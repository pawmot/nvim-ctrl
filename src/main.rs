use anyhow::Result;
use lazy_static::lazy_static;
use neovim_lib::{Neovim, NeovimApi, Session};
use regex::Regex;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Control nvim from the CLI!")]
struct Control {
    /// run an arbitrary command
    cmd: String,
}

fn main() -> Result<()> {
    let args = Control::from_args();
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR");

    let dirs = match xdg_runtime_dir {
        Ok(dir) => vec![dir],
        Err(_) => {
            let tmp = std::env::var("TMPDIR").unwrap_or("/tmp".to_owned());
            let user = std::env::var("USER");
            if user.is_err() {
                panic!("Could not get user name");
            }
            let nvim_dir = format!("{}/nvim.{}", tmp, user.unwrap());
            match std::fs::read_dir(nvim_dir) {
                Ok(dir) => dir
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    })
                    .map(|entry| entry.path().to_string_lossy().to_string())
                    .collect(),
                Err(_) => panic!("Could not find nvim socket"),
            }
        }
    };

    lazy_static! {
        static ref NVIM_RPC_SOCKET_RE: Regex =
            Regex::new(r"^nvim.\d+.0$").unwrap();
    }

    dirs.into_iter()
        .map(std::fs::read_dir)
        .filter_map(|entry| entry.ok())
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            NVIM_RPC_SOCKET_RE
                .is_match(entry.file_name().to_string_lossy().as_ref())
        })
        .map(|entry| entry.path())
        .filter_map(|path| Session::new_unix_socket(path).ok())
        .map(|mut session| {
            session.start_event_loop();
            Neovim::new(session)
        })
        .for_each(|mut nvim| {
            let _ = nvim
                .command(&args.cmd)
                .map_err(|e| eprintln!("Error: {}", e));
        });

    Ok(())
}
