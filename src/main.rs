use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::errors::ReplyError;
use x11rb::rust_connection::RustConnection;


use std::process::{Command, Stdio};
use std::io::BufRead;


use clap::{Parser, Subcommand};
use clap_num::maybe_hex;


/// Utility functions to manipulate a tabbed window.
/// All input window ids can be in decimal, hex with the prefix "0x", or the string "focused" to
/// apply to the currently focused window.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Reparent a set of windows to a tabbed instance, creating one if necessary
    Create {
        /// Window IDs to combine into a tabbed instance
        #[arg()]
        wids: Vec<String>,
    },
    /// Detach from a tabbed container; by default, detaches active window only
    Detach {
        /// Window to detach from, expected to be a tabbed instance, no-op otherwise
        #[arg()]
        wid: String,

        /// Detach all children of the window instead of only active; deletes the tabbed instance
        #[arg(short,long)]
        all: bool,
    },
    /// Embed the next opened program with the target window
    Embed {
        /// Target window to autoattach to once
        #[arg()]
        wid: String,
    },
}




fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    match cli.command {
        Commands::Create { wids } => {
            create(&conn, resolve_wids(wids)?)?;
        },
        Commands::Detach { wid, all: true } => {
            detach_all(&conn, resolve_wid(&wid)?, root)?;
        },
        Commands::Detach { wid, all: false } => {
            detach_current(&conn, resolve_wid(&wid)?, root)?;
        },
        Commands::Embed { wid } => {
            embed(&conn, resolve_wid(&wid)?)?;
        },
    }

    conn.flush()?;
    Ok(())
}


fn create(conn: &RustConnection, wid: Vec<Window>) -> Result<Window, ReplyError> {
    let mut to_reparent = Vec::new();
    let mut last_tabbed: Option<Window> = None;

    for w in wid {
        if is_tabbed(conn, w)? {
            if let Some(last_w) = last_tabbed {
                let mut last_q = query_tree(conn, last_w)?.reply()?;
                to_reparent.append(&mut last_q.children);
            }
            last_tabbed = Some(w);
        } else {
            bspc_disable_border(w);
            to_reparent.push(w);
        }
    }

    // If a tabbed instance was in the list, use it. Otherwise, spawn a new tabbed and use that
    let tabbed_window = last_tabbed.unwrap_or_else(create_tabbed);

    for w in &to_reparent {
        reparent_window(conn, *w, tabbed_window, 0, 0)?.check()?;
    }

    // sometimes the tabs get a bit "stuck". Reparenting them all back seems to fix it
    detach_all(conn, tabbed_window, tabbed_window)?;

    Ok(tabbed_window)
}


fn embed(conn: &RustConnection, wid: Window) -> Result<(), ReplyError> {
    let tabbed_window = create(conn, vec![wid])?;

    let child = Command::new("bspc")
        .args(["subscribe", "node_add"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let output = child.stdout.unwrap();
    let mut lines = std::io::BufReader::new(output).lines();

    if let Some(Ok(text)) = lines.next() {
        let parts: Vec<_> = text.split_whitespace().collect();
        let id_str = parts[4].strip_prefix("0x").unwrap().trim();
        let new_wid = Window::from_str_radix(id_str, 16).unwrap();

        bspc_disable_border(new_wid);
        reparent_window(conn, new_wid, tabbed_window, 0, 0)?.check()?;
    }

    Ok(())
}


fn is_tabbed(conn: &RustConnection, wid: Window) -> Result<bool, ReplyError> {
    let prop = get_property(conn, false, wid, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 8);
    Ok(prop?.reply()?.value == b"tabbed\0tabbed\0")
}


fn detach_all(conn: &RustConnection, wid: Window, root: Window) -> Result<(), ReplyError> {
    let q = query_tree(conn, wid)?.reply()?;
    if q.length > 1 {
        for w in q.children {
            reparent_window(conn, w, root, 0, 0)?.check()?;
        }
    }

    Ok(())
}


fn detach_current(conn: &RustConnection, wid: Window, root: Window) -> Result<(), ReplyError> {
    let q = query_tree(conn, wid)?.reply()?;
    if let Some(&first) = q.children.last() {
        reparent_window(conn, first, root, 0, 0)?.check()?;
    }

    Ok(())
}




fn create_tabbed() -> Window {
    let child = Command::new("tabbed")
        .args(["-c", "-d"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let output = child.wait_with_output().unwrap();
    let id_str = std::str::from_utf8(&output.stdout).unwrap()
        .strip_prefix("0x").unwrap().trim();

    Window::from_str_radix(id_str, 16).unwrap()
}


fn resolve_wids(wids: Vec<String>) -> Result<Vec<Window>, String> {
        wids.into_iter().map(|wid| resolve_wid(&wid)).collect()
}

fn resolve_wid(wid: &str) -> Result<Window, String> {
    maybe_hex(wid).or_else(|_| bspc_query_node(wid))
}

fn bspc_disable_border(wid: Window) {
    Command::new("bspc")
        .args(["config", "-n", &wid.to_string(), "border_width", "0"])
        .status()
        .expect("failed to execute bspc config");
}

fn bspc_query_node(node_sel: &str) -> Result<Window, String> {
    let out = Command::new("bspc")
        .args(["query", "-N", "-n", node_sel])
        .output()
        .expect("failed to execute bspc query");
    
    if out.stderr != b"" {
        let err = String::from_utf8(out.stderr).unwrap();
        let err = err.strip_prefix("query -n: ").unwrap_or(&err).trim();
        return Err(err.to_string());
    }

    if out.stdout == b"" {
        return Err(format!("Descriptor matched no nodes: '{}'.", node_sel));
    }

    let id_str = std::str::from_utf8(&out.stdout).unwrap()
        .strip_prefix("0x").unwrap().trim();

    Window::from_str_radix(id_str, 16).map_err(|e| format!("{e}"))
}
