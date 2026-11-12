// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The DISLED contributors

//! Update protocol of the DISLED HID bootloader
//! (firmware/bootloader/README.md): 64-byte reports without report ID, one
//! reply for each request. The functions here do no I/O: a [`Transport`]
//! carries the reports, hidapi in the tool, the bootloader's own C core in
//! the tests.

use std::fmt;

/// USB IDs of the bootloader. 0x0003 is a pid.codes test PID, for private
/// testing only: the bootloader needs a PID of its own before it ships.
pub const VENDOR_ID: u16 = 0x1209;
pub const PRODUCT_ID: u16 = 0x0003;
pub const USAGE_PAGE: u16 = 0xFF00;

pub const REPORT_SIZE: usize = 64;
const DATA_OFFSET: usize = 8;
const REPLY_OFFSET: usize = 4;
const PAGE_SIZE: usize = 256;
const PROTOCOL_MAJOR: u8 = 1;

/// Why the bootloader stayed in update mode (`Info::reasons`).
pub const REASON_REQUESTED: u8 = 1 << 0;
pub const REASON_APP_INVALID: u8 = 1 << 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Info = 0x01,
    Begin = 0x02,
    Data = 0x03,
    Program = 0x04,
    Finish = 0x05,
    Boot = 0x06,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The bootloader answered with an error status.
    Status(Command, u8),
    /// No reply, or one that does not answer the request.
    Transport(String),
    /// The image does not suit this bootloader.
    Image(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Status(command, status) => {
                let reason = match status {
                    1 => "unknown command",
                    2 => "argument out of range",
                    3 => "no update under way",
                    4 => "the flash does not read back as written (write-protected?)",
                    5 => "CRC mismatch: the image in flash is not the one sent",
                    _ => "unknown error",
                };
                write!(f, "{command:?} failed: {reason} (status {status})")
            }
            Error::Transport(message) | Error::Image(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

/// Carries one request and returns the reply that answers it.
pub trait Transport {
    fn transact(&mut self, request: &[u8; REPORT_SIZE]) -> Result<[u8; REPORT_SIZE], String>;
}

/// The validity record of the image in flash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub size: u32,
    pub crc: u32,
    pub tag: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    pub protocol: [u8; 2],
    pub reasons: u8,
    pub app_offset: u32,
    pub app_max: u32,
    pub page_size: u16,
    pub data_max: u8,
    pub record: Option<Record>,
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

impl Info {
    fn decode(body: &[u8]) -> Info {
        let size = u32_at(body, 16);
        Info {
            protocol: [body[0], body[1]],
            reasons: body[2],
            app_offset: u32_at(body, 4),
            app_max: u32_at(body, 8),
            page_size: u16::from_le_bytes([body[12], body[13]]),
            data_max: body[14],
            record: (size != 0).then(|| Record {
                size,
                crc: u32_at(body, 20),
                tag: u32_at(body, 24),
            }),
        }
    }
}

/// Request layout: command, seq, data length, 0, u32 argument, data.
pub fn encode(command: Command, seq: u8, arg: u32, data: &[u8]) -> [u8; REPORT_SIZE] {
    assert!(data.len() <= REPORT_SIZE - DATA_OFFSET);
    let mut report = [0u8; REPORT_SIZE];
    report[0] = command as u8;
    report[1] = seq;
    report[2] = data.len() as u8;
    report[4..8].copy_from_slice(&arg.to_le_bytes());
    report[DATA_OFFSET..DATA_OFFSET + data.len()].copy_from_slice(data);
    report
}

/// CRC-32 of zlib, as the bootloader computes it.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

/// A connection to the bootloader.
pub struct Session<T: Transport> {
    transport: T,
    seq: u8,
}

impl<T: Transport> Session<T> {
    pub fn new(transport: T) -> Self {
        Session { transport, seq: 0 }
    }

    fn request(
        &mut self,
        command: Command,
        arg: u32,
        data: &[u8],
    ) -> Result<[u8; REPORT_SIZE], Error> {
        self.seq = self.seq.wrapping_add(1);
        let reply = self
            .transport
            .transact(&encode(command, self.seq, arg, data))
            .map_err(Error::Transport)?;
        if reply[0] != command as u8 || reply[1] != self.seq {
            return Err(Error::Transport(format!(
                "reply {:02x}/{} to {command:?}/{}",
                reply[0], reply[1], self.seq
            )));
        }
        match reply[2] {
            0 => Ok(reply),
            status => Err(Error::Status(command, status)),
        }
    }

    pub fn info(&mut self) -> Result<Info, Error> {
        let reply = self.request(Command::Info, 0, &[])?;
        let info = Info::decode(&reply[REPLY_OFFSET..]);
        if info.protocol[0] != PROTOCOL_MAJOR {
            return Err(Error::Image(format!(
                "bootloader protocol {}.{}, this tool speaks {PROTOCOL_MAJOR}.x",
                info.protocol[0], info.protocol[1]
            )));
        }
        Ok(info)
    }

    /// Writes `image`, linked at the application offset, then its validity
    /// record. `progress` gets the bytes written and the total.
    pub fn update(
        &mut self,
        image: &[u8],
        tag: u32,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<(), Error> {
        let info = self.info()?;
        if image.is_empty() || image.len() > info.app_max as usize {
            return Err(Error::Image(format!(
                "image of {} bytes, the bootloader takes 1 to {}",
                image.len(),
                info.app_max
            )));
        }
        if usize::from(info.page_size) != PAGE_SIZE || info.data_max == 0 {
            return Err(Error::Image(format!(
                "unexpected geometry: page {}, data {}",
                info.page_size, info.data_max
            )));
        }
        // The startup of the WCH SDK puts the vector table right after the
        // 4-byte reset jump, and its first entry is `_start`: the address
        // the image was linked at. Catches a firmware built for address 0.
        let linked_at = if image.len() >= 8 {
            u32_at(image, 4)
        } else {
            0
        };
        if linked_at != info.app_offset {
            return Err(Error::Image(format!(
                "the image is linked at {linked_at:#x}, not {:#x}: build it with the \
                 release_bl or debug_bl environment",
                info.app_offset
            )));
        }

        self.request(Command::Begin, 0, &[])?;
        let chunk = usize::from(info.data_max);
        for (index, part) in image.chunks(PAGE_SIZE).enumerate() {
            let mut page = [0xFFu8; PAGE_SIZE];
            page[..part.len()].copy_from_slice(part);
            for offset in (0..PAGE_SIZE).step_by(chunk) {
                let end = (offset + chunk).min(PAGE_SIZE);
                self.request(Command::Data, offset as u32, &page[offset..end])?;
            }
            self.request(Command::Program, index as u32, &[])?;
            progress(index * PAGE_SIZE + part.len(), image.len());
        }
        let mut finish = [0u8; 8];
        finish[..4].copy_from_slice(&crc32(image).to_le_bytes());
        finish[4..].copy_from_slice(&tag.to_le_bytes());
        self.request(Command::Finish, image.len() as u32, &finish)?;
        Ok(())
    }

    /// The bootloader replies, then resets into the application if valid.
    pub fn boot(&mut self) -> Result<(), Error> {
        self.request(Command::Boot, 0, &[]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    //! The host side against the bootloader's own C core (build.rs), on a
    //! model of the 32 KB flash: the protocol end to end, without a board.

    use super::*;
    use std::cell::UnsafeCell;
    use std::sync::Mutex;

    const FLASH_SIZE: usize = 0x8000;
    const APP_OFFSET: usize = 0x1000;
    const RECORD_OFFSET: usize = FLASH_SIZE - PAGE_SIZE;
    const ERASED: [u8; 4] = 0xE339_E339u32.to_le_bytes();

    /// The flash model, global like the state of the C core.
    struct Flash(UnsafeCell<[u8; FLASH_SIZE]>);
    // SAFETY: only used by the tests that hold LOCK.
    unsafe impl Sync for Flash {}
    static FLASH: Flash = Flash(UnsafeCell::new([0; FLASH_SIZE]));
    /// One test at a time.
    static LOCK: Mutex<()> = Mutex::new(());

    unsafe extern "C" {
        fn bl_proto_handle(flash: *const u8, request: *const u8, reply: *mut u8) -> bool;
        fn bl_boot_reasons(flash: *const u8, requested: bool) -> u8;
    }

    fn flash() -> &'static mut [u8; FLASH_SIZE] {
        // SAFETY: the tests hold LOCK; the C core only reads through the
        // pointer it gets, between the calls of the model below.
        unsafe { &mut *FLASH.0.get() }
    }

    fn flash_ptr() -> *const u8 {
        FLASH.0.get().cast()
    }

    /// The flash port of the C core: page erase, then program.
    #[unsafe(no_mangle)]
    extern "C" fn bl_port_write_page(offset: u32, data: *const u32) -> bool {
        let offset = offset as usize;
        if offset < APP_OFFSET
            || offset + PAGE_SIZE > FLASH_SIZE
            || !offset.is_multiple_of(PAGE_SIZE)
        {
            return false; // the protected bootloader, or a bug
        }
        let page = &mut flash()[offset..offset + PAGE_SIZE];
        for word in page.chunks_mut(4) {
            word.copy_from_slice(&ERASED);
        }
        if !data.is_null() {
            // SAFETY: the core passes a 256-byte page.
            page.copy_from_slice(unsafe {
                std::slice::from_raw_parts(data.cast::<u8>(), PAGE_SIZE)
            });
        }
        true
    }

    fn erase_all() {
        for word in flash().chunks_mut(4) {
            word.copy_from_slice(&ERASED);
        }
    }

    fn boot_reasons(requested: bool) -> u8 {
        unsafe { bl_boot_reasons(flash_ptr(), requested) }
    }

    struct Core {
        requests: usize,
        boot: bool,
    }

    impl Transport for Core {
        fn transact(&mut self, request: &[u8; REPORT_SIZE]) -> Result<[u8; REPORT_SIZE], String> {
            let mut reply = [0u8; REPORT_SIZE];
            self.requests += 1;
            self.boot |=
                unsafe { bl_proto_handle(flash_ptr(), request.as_ptr(), reply.as_mut_ptr()) };
            Ok(reply)
        }
    }

    /// An application as the WCH startup lays it out at 0x1000: the reset
    /// jump, then the vector table, whose first entry is `_start`.
    fn image(size: usize) -> Vec<u8> {
        let mut image: Vec<u8> = (0..size).map(|i| (i * 31 + (i >> 8)) as u8).collect();
        image[..4].copy_from_slice(&0x48E0_406Fu32.to_le_bytes()); // j handle_reset
        image[4..8].copy_from_slice(&(APP_OFFSET as u32).to_le_bytes());
        image
    }

    #[test]
    fn crc32_vectors() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn encode_layout() {
        let report = encode(Command::Data, 7, 0x0102_0304, &[0xAA, 0xBB]);
        assert_eq!(
            &report[..10],
            &[0x03, 7, 2, 0, 0x04, 0x03, 0x02, 0x01, 0xAA, 0xBB]
        );
        assert!(report[10..].iter().all(|&b| b == 0));
    }

    #[test]
    fn update_end_to_end() {
        let _guard = LOCK.lock().unwrap();
        erase_all();
        assert_eq!(boot_reasons(false), REASON_APP_INVALID);

        let mut session = Session::new(Core {
            requests: 0,
            boot: false,
        });
        let info = session.info().unwrap();
        assert_eq!(info.protocol, [1, 0]);
        assert_eq!(info.reasons, REASON_APP_INVALID);
        assert_eq!((info.app_offset, info.app_max), (0x1000, 28416));
        assert_eq!((info.page_size, info.data_max), (256, 56));
        assert_eq!(info.record, None);

        let app = image(18468);
        let mut last = 0;
        session
            .update(&app, 0x0001_0000, |done, total| {
                assert!(done > last && total == app.len());
                last = done;
            })
            .unwrap();
        assert_eq!(last, app.len());
        assert_eq!(&flash()[APP_OFFSET..APP_OFFSET + app.len()], &app[..]);
        assert_eq!(
            boot_reasons(false),
            0,
            "the bootloader starts the application"
        );

        let info = session.info().unwrap();
        assert_eq!(
            info.record,
            Some(Record {
                size: 18468,
                crc: crc32(&app),
                tag: 0x0001_0000
            })
        );
        session.boot().unwrap();
        assert!(session.transport.boot);
        // INFO twice, BEGIN, 73 pages of 5 DATA and 1 PROGRAM, FINISH, INFO,
        // BOOT.
        assert_eq!(session.transport.requests, 2 + 1 + 73 * 6 + 1 + 1 + 1);
    }

    #[test]
    fn interrupted_update_resumes() {
        let _guard = LOCK.lock().unwrap();
        erase_all();
        let mut session = Session::new(Core {
            requests: 0,
            boot: false,
        });
        session.update(&image(5000), 1, |_, _| {}).unwrap();
        assert_eq!(boot_reasons(false), 0);

        // The transport dies in the middle of the next update.
        struct Dying(Core, usize);
        impl Transport for Dying {
            fn transact(&mut self, r: &[u8; REPORT_SIZE]) -> Result<[u8; REPORT_SIZE], String> {
                if self.0.requests == self.1 {
                    return Err("unplugged".into());
                }
                self.0.transact(r)
            }
        }
        let mut dying = Session::new(Dying(
            Core {
                requests: 0,
                boot: false,
            },
            20,
        ));
        let error = dying.update(&image(9000), 2, |_, _| {}).unwrap_err();
        assert_eq!(error, Error::Transport("unplugged".into()));
        // After the power cycle: update mode, then a full update works.
        assert_eq!(boot_reasons(false), REASON_APP_INVALID);
        let mut session = Session::new(Core {
            requests: 0,
            boot: false,
        });
        session.update(&image(9000), 2, |_, _| {}).unwrap();
        assert_eq!(boot_reasons(false), 0);
    }

    #[test]
    fn rejects_bad_images() {
        let _guard = LOCK.lock().unwrap();
        erase_all();
        let mut session = Session::new(Core {
            requests: 0,
            boot: false,
        });
        assert!(matches!(
            session.update(&[], 0, |_, _| {}),
            Err(Error::Image(_))
        ));
        assert!(matches!(
            session.update(&image(28417), 0, |_, _| {}),
            Err(Error::Image(_))
        ));
        let mut linked_at_zero = image(1000);
        linked_at_zero[4..8].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            session.update(&linked_at_zero, 0, |_, _| {}),
            Err(Error::Image(_))
        ));
        // Nothing was erased: the bootloader still has no record to lose.
        assert_eq!(flash()[RECORD_OFFSET..RECORD_OFFSET + 4], ERASED);
    }
}
