use std::env;
use std::fs;
use std::process;

use memcard::fat::Memcard;

fn main() -> std::io::Result<()> {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example inspect -- <path-to-memcard>");
        process::exit(1);
    });

    let data = fs::read(&path)?;

    let mut mc = Memcard::new(data);

    let root_entries = mc.read_entry_cluster(mc.rootdir_cluster());
    if let Some(root) = root_entries.first() {
        eprintln!("{:#?}", root);
    } else {
        eprintln!("No entries found in root directory.");
    }

    mc.print_allocation_table_recursive();

    Ok(())
}
