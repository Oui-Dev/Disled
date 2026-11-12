# Claude Code plugin

An example of a plugin with events ([docs/plugins.md](../../../docs/plugins.md)):
the lamp shows what [Claude Code](https://code.claude.com/docs/en/overview)
does.

| State | Color | When |
|---|---|---|
| Claude is working | red | you send a prompt, a tool runs |
| Claude waits for your answer | orange, with an alert | Claude asks you a question, a permission or the approval of a plan, or a request fails |
| Claude is done | green | Claude ends its turn |

The integrations window lets you give each state another color.

## Install

You need Disled 2.0 or later.

1. Copy this `claude-code` folder into the plugins folder of Disled: in the
   popup, the mode button, **Manage integrations**, then **Open the plugins
   folder**. Or zip the folder and choose **Import a plugin**.
2. Turn **Claude Code** on in the integrations window, and accept what it
   asks for.
3. Its installation note shows a block of hooks, with the address and the
   token filled in. **Copy** it into `~/.claude/settings.json`, in your home
   folder. If the file has a `hooks` key already, add these entries to it
   without removing yours. Claude Code takes them into account without a
   restart.
4. Choose **Claude Code** in the mode menu of the popup.

Disled never changes the settings of Claude Code: you paste the block
yourself.

## How it works

Claude Code sends each [hook](https://code.claude.com/docs/en/hooks) event
to Disled on `127.0.0.1`, with the plugin's token. The rules of [`plugin.json`](plugin.json) turn the events
into states, the first rule that matches winning:

| Event | State |
|---|---|
| `SessionEnd` | the session leaves |
| `PreToolUse` of `AskUserQuestion` or `ExitPlanMode`, `PermissionRequest`, `Elicitation`, `StopFailure` | waiting, with an alert |
| `Stop` while background tasks still run | working |
| `Stop` | done |
| `Notification` | ignored |
| `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `ElicitationResult` | working |

Each session of Claude Code (`session_id`) keeps its own state, and the
lamp shows the most urgent one: waiting, then working, then done. A session
that sends nothing for a while leaves: after 5 minutes when it is working,
because Claude Code sends nothing when you interrupt a turn with Escape, and
after 30 minutes otherwise. Without a session left, the lamp shows "Claude
is done".

Behind a proxy, leave `127.0.0.1` out of it
([docs/plugins.md](../../../docs/plugins.md#behind-a-proxy)).
