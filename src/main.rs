use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::errors::ReplyError;
use x11rb::rust_connection::RustConnection;


use std::process::{Command, Stdio};
use std::io::BufRead;


use clap::{Parser, Subcommand};
use clap_num::maybe_hex;

use nonempty::{NonEmpty, nonempty};



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
        // Window IDs to combine into a tabbed instance
        #[arg(num_args=1..)]
        wids: Vec<String>,
    },
    /// Attach window <WID0> to tabbed <WID1>.
    ///
    /// If <WID0> is tabbed, use the active window instead.
    /// If <WID1> is not tabbed, call `create <WID1>` first.
    Transfer {
        #[arg()]
        wid0: String,
        #[arg()]
        wid1: String,
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
    /// Reparent all children of a node back to itself.
    ///
    /// tabbed is rather buggy, and it's hard to guarantee this won't ever be necessary.
    Fix {
        #[arg()]
        wid: String,
    },
    Query {
        #[arg()]
        wid: String,
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
            let wids = NonEmpty::from_vec(resolve_wids(&wids)?)
                .expect("create args cannot be empty");
            create(&conn, wids)?;
        },
        Commands::Transfer { wid0, wid1 } => {
            transfer(&conn, resolve_wid(&wid0)?, resolve_wid(&wid1)?, root)?;
        },
        Commands::Detach { wid, all: true } => {
            reparent_all(&conn, resolve_wid(&wid)?, root)?;
        },
        Commands::Detach { wid, all: false } => {
            detach_current(&conn, resolve_wid(&wid)?, root)?;
        },
        Commands::Fix { wid } => {
            let wid = resolve_wid(&wid)?;
            reparent_all(&conn, wid, wid)?;
        },
        Commands::Query { wid } => {
            let wid = resolve_wid(&wid)?;
            query(&conn, wid)?;
        },
        Commands::Embed { wid } => {
            embed(&conn, resolve_wid(&wid)?)?;
        },
    }

    conn.flush()?;
    Ok(())
}


fn create(conn: &RustConnection, wids: NonEmpty<Window>) -> Result<Window, ReplyError> {
    let mut to_reparent = Vec::new();

    for &w in wids.iter().take(wids.len() - 1) {
        if is_tabbed(conn, w)? {
            let mut q = query_tree(conn, w)?.reply()?;
            to_reparent.append(&mut q.children);
        } else {
            bspc_disable_border(&w.to_string());
            to_reparent.push(w);
        }
    }

    let &last = wids.last();
    bspc_focus(last);

    // If the last window is tabbed, use it. Otherwise, spawn a new tabbed and use that
    let tabbed = if is_tabbed(conn, last)? {
        last
    } else {
        bspc_disable_border(&last.to_string());
        let t = create_tabbed();
        reparent_window(conn, last, t, 0, 0)?.check()?;
        t
    };

    //for &w in to_reparent.iter().rev() {
    for &w in &to_reparent {
        reparent_window(conn, w, tabbed, 0, 0)?.check()?;
    }

    conn.flush()?;

    // Sometimes the tabs get a bit "stuck". Reparenting them all back seems to fix it
    if query_tree(conn, tabbed)?.reply()?.children_len() > 1 {
        reparent_all(conn, tabbed, tabbed)?;
    }

    Ok(tabbed)
}


fn transfer(conn: &RustConnection, wid0: Window, wid1: Window, root: Window) -> Result<(), ReplyError> {
    let wid0 = detach_current(conn, wid0, root)?.unwrap_or(wid0);
    let _ = create(conn, nonempty![
           wid0, wid1
    ])?;
    Ok(())
}


fn embed(conn: &RustConnection, wid: Window) -> Result<(), ReplyError> {
    bspc_focus(wid);
    let tabbed_window = create(conn, nonempty![wid])?;

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

        bspc_disable_border(&new_wid.to_string());
        reparent_window(conn, new_wid, tabbed_window, 0, 0)?.check()?;
    }

    Ok(())
}


fn is_tabbed(conn: &RustConnection, wid: Window) -> Result<bool, ReplyError> {
    let prop = get_property(conn, false, wid, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 8);
    Ok(prop?.reply()?.value == b"tabbed\0tabbed\0")
}


fn reparent_all(conn: &RustConnection, wid0: Window, wid1: Window) -> Result<Vec<Window>, ReplyError> {
    let q = query_tree(conn, wid0)?.reply()?;

    for &w in &q.children {
        reparent_window(conn, w, wid1, 0, 0)?.check()?;
    }

    Ok(q.children)
}


//// this would be faster/better than detach_current in most cases, but tabbed is terribly buggy
//fn reparent_current(conn: &RustConnection, wid0: Window, wid1: Window) -> Result<Option<Window>, ReplyError> {
//    let q = query_tree(conn, wid0)?.reply()?;
//    if let Some(&active) = q.children.last() {
//        reparent_window(conn, active, wid1, 0, 0)?.check()?;
//        Ok(Some(active))
//    } else {
//        Ok(None)
//    }
//}

fn detach_current(conn: &RustConnection, wid: Window, root: Window) -> Result<Option<Window>, ReplyError> {
    // tabbed doesn't properly deal with reparenting away from itself so we detach all and make a
    // new one. Hopefully there's some way to not have to do this BS. Tabbed is only around a
    // thousand LoC, maybe I could try to just fix it.
    
    let children = reparent_all(conn, wid, root)?;

    if let Some((&active, rest)) = children.split_last() {
        if !rest.is_empty() {
            bspc_disable_border(&(rest[0].to_string() + "#first_ancestor"));
            let tabbed_window = create_tabbed();
            for &child in rest {
                reparent_window(conn, child, tabbed_window, 0, 0)?.check()?;
            }
        }

        reparent_window(conn, active, root, 0, 0)?.check()?;

        Ok(Some(active))
    } else {
        Ok(None)
    }
}


fn query(conn: &RustConnection, wid: Window) -> Result<(), ReplyError> {
    println!("wid: 0x{:X} {}", wid, wid);
    println!("is_tabbed: {}", is_tabbed(conn, wid)?);
    println!("children: {:?}", query_tree(conn, wid)?.reply()?.children);
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


fn resolve_wids(wids: &Vec<String>) -> Result<Vec<Window>, String> {
        wids.iter().map(|wid| resolve_wid(wid)).collect()
}

fn resolve_wid(wid: &str) -> Result<Window, String> {
    maybe_hex(wid).or_else(|_| bspc_query_node(wid))
}


fn bspc_disable_border(node_sel: &str) {
    Command::new("bspc")
        .args(["config", "-n", node_sel, "border_width", "0"])
        .status()
        .expect("failed to execute bspc config");
}

fn bspc_focus(wid: Window) {
    Command::new("bspc")
        .args(["node", &wid.to_string(), "--focus"])
        .status()
        .expect("failed to execute bspc node");
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
