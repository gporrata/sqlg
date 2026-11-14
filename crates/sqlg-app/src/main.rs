mod db;
mod files;
mod ipc;
mod logic;
mod message;
mod model;
mod runtime;
mod theme;
mod update;
mod view;

fn main() {
    let dir = sqlg_core::data_dir();
    if let ipc::Acquire::Primary(primary) = ipc::acquire(&dir) {
        runtime::launch(primary);
    }
}
