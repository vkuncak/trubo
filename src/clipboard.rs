use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

pub fn set_text(text: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return write_to_command("pbcopy", &[], text);
    }

    #[cfg(target_os = "windows")]
    {
        return write_to_command(
            "powershell",
            &["-NoProfile", "-Command", "Set-Clipboard"],
            text,
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let commands: [(&str, &[&str]); 3] = [
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ];
        return first_successful_write(&commands, text);
    }

    #[allow(unreachable_code)]
    Err(io::Error::other(
        "clipboard is not supported on this platform",
    ))
}

pub fn get_text() -> io::Result<String> {
    #[cfg(target_os = "macos")]
    {
        return read_from_command("pbpaste", &[]);
    }

    #[cfg(target_os = "windows")]
    {
        return read_from_command(
            "powershell",
            &["-NoProfile", "-Command", "Get-Clipboard -Raw"],
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let commands: [(&str, &[&str]); 3] = [
            ("wl-paste", &[]),
            ("xclip", &["-selection", "clipboard", "-out"]),
            ("xsel", &["--clipboard", "--output"]),
        ];
        return first_successful_read(&commands);
    }

    #[allow(unreachable_code)]
    Err(io::Error::other(
        "clipboard is not supported on this platform",
    ))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn first_successful_write(commands: &[(&str, &[&str])], text: &str) -> io::Result<()> {
    let mut last_error = io::Error::other("no clipboard command configured");
    for (program, args) in commands {
        match write_to_command(program, args, text) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn first_successful_read(commands: &[(&str, &[&str])]) -> io::Result<String> {
    let mut last_error = io::Error::other("no clipboard command configured");
    for (program, args) in commands {
        match read_from_command(program, args) {
            Ok(text) => return Ok(text),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

fn write_to_command(program: &str, args: &[&str], text: &str) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{program} exited with status {status}"
        )))
    }
}

fn read_from_command(program: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(program).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(io::Error::other(format!(
            "{program} exited with status {}",
            output.status
        )))
    }
}
