use std::process::{Command, ExitCode, ExitStatus, Stdio};
use std::{env, fs, io};
use std::fs::Permissions;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use nix::sched::{CloneFlags, unshare};
use nix::mount;
use nix::mount::MsFlags;
use nix::unistd::{pivot_root, chdir};


enum ProcessMode {
    Run,
    Child,
}

impl ProcessMode {
    fn from_string(string: &str) -> Option<Self> {
        match string {
            "run" => Some(ProcessMode::Run),
            "child" => Some(ProcessMode::Child),
            _ => None
        }
    }
}


fn main() -> io::Result<ExitCode> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        let exec_name = args[0].as_str();
        panic!("{}", format!("{exec_name} needs at least one argument."))
    }
    let mode = ProcessMode::from_string(args[1].as_str()).expect(&format!("{} bad command line argument", args[1].as_str()));
    let exit_code = match mode {
        ProcessMode::Run => {
            parent(&args)
        }
        ProcessMode::Child => {
            child(&args)
        }
    }?.code();

    match exit_code {
        Some(code) => Ok((code as u8).into()),
        None => {
            Err(io::Error::from(ErrorKind::Other))
        }
    }
}

fn parent(args: &Vec<String>) -> io::Result<ExitStatus> {
    Command::new("/proc/self/exe")
        .arg("child")
        .args(&args[2..])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .spawn()?.wait()
}

fn child(args: &Vec<String>) -> io::Result<ExitStatus> {
    println!("here1");

    let clone_flags =
        CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWIPC
            | CloneFlags::CLONE_NEWNET
            | CloneFlags::CLONE_NEWUSER;

    unshare(clone_flags)?;
    println!("here2");
    //rootfs MUST be some other filesystem.
    fs::create_dir_all("rootfs")?;
    fs::set_permissions("rootfs", Permissions::from_mode(0o700))?;
    Command::new("mount").arg("-o").arg("loop").arg("alpine-x86_64-lts.img").arg("rootfs").output()?;
    // mount::mount(Some("alpine-x86_64-lts.img"), "rootfs", Option::<&str>::None, MsFlags::MS_BIND, Option::<&str>::None)?;
    println!("here3");
    Command::new("mount").arg("-t").arg("tmpfs").arg("tmpfs").arg("rootfs/oldrootfs").output()?;
    println!("here5");
    pivot_root("rootfs", "rootfs/oldrootfs")?;
    println!("here6");
    chdir("/")?;
    let result = Command::new(&args[2])
        .args(&args[3..])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .spawn()?
        .wait();
    Command::new("umount").arg("rootfs/oldrootfs").output()?;
    Command::new("umount").arg("rootfs").output()?;
    result
}
