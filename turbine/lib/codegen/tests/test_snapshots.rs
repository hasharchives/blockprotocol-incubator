use std::{fs, path::Path};

use serde_json::Value;

#[test]
fn snapshots() {
    let location = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    let mut snapshots = vec![];

    // find all snapshots
    for entry in fs::read_dir(location).expect("should be able to read dir") {
        let entry = entry.expect("should be able to read entries in `snapshots/` directory");

        let file_type = entry.file_type().expect("unable to determine file type");

        if file_type.is_dir() {
            continue;
        }

        if entry.path().ends_with(".json") {
            snapshots.push(entry.path());
        }
    }

    let overwrite = env!("SNAPSHOT_MODE").to_ascii_lowercase() == "overwrite";

    for snapshot in snapshots {
        let snapshot = fs::read_to_string(&snapshot).expect("unable to read snapshot");
        let contents =
            serde_json::from_str::<Vec<Value>>(&snapshot).expect("snapshot is invalid JSON");

        let output = codegen::process(contents).expect("able to generate valid rust");

        todo!()
    }
}
