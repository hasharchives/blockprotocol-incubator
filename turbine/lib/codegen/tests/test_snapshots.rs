use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use serde_json::Value;
use similar_asserts::assert_eq;

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

    for path in snapshots {
        let snapshot = fs::read_to_string(&path).expect("unable to read snapshot");
        let contents =
            serde_json::from_str::<Vec<Value>>(&snapshot).expect("snapshot is invalid JSON");

        let output = codegen::process(contents).expect("able to generate valid rust");

        let output = output
            .into_iter()
            .map(|(file, stream)| {
                let mut command = Command::new("rustfmt")
                    .arg("--emit")
                    .arg("stdout")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("able to spawn rustfmt");

                command
                    .stdin
                    .take()
                    .expect("stdio piped")
                    .write(stream.to_string().as_bytes())
                    .expect("should be able to write to stdin");
                let output = command.wait_with_output().unwrap();

                let output = String::from_utf8(output.stdout).unwrap();
                let path = file.path;

                format!("{path} \n\n {output}")
            })
            .reduce(|mut acc, next| {
                acc.push_str("\n\n---\n\n");
                acc.push_str(&next);
                acc
            })
            .expect("no files");

        let expected = path.with_extension(".stdout");
        if overwrite {
            fs::write(expected, output).unwrap();
        } else {
            let expected = fs::read_to_string(&expected).unwrap();

            assert_eq!(output, expected);
        }
    }
}
