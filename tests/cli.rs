use std::{fs, process::Command};

fn minimal_scalar_json() -> &'static str {
    r#"{
      "schema_version": 1,
      "document": {
        "structure": {
          "root": "0",
          "next_id": "2",
          "arena": [
            {"id":"0","kind":{"kind":"Sequence","children":["1"]}},
            {"id":"1","kind":{"kind":"Scalar","name":"N"}}
          ]
        },
        "constraints": {
          "next_id": "2",
          "arena": [
            {"id":"0","constraint":{"kind":"TypeDecl","target":{"kind":"VariableRef","node_id":"1"},"expected":"Int"}},
            {"id":"1","constraint":{"kind":"Range","target":{"kind":"VariableRef","node_id":"1"},"lower":{"kind":"Lit","value":1},"upper":{"kind":"Lit","value":3}}}
          ],
          "by_node": [{"node_id":"1","constraints":["0","1"]}],
          "global": []
        }
      }
    }"#
}

fn encoded_state() -> String {
    urlencoding::encode(minimal_scalar_json()).into_owned()
}

#[test]
fn same_seed_generates_same_output() {
    let bin = env!("CARGO_BIN_EXE_rt");
    let state = encoded_state();
    let first = Command::new(bin)
        .arg(&state)
        .arg("--seed")
        .arg("1")
        .output()
        .expect("rt should run");
    let second = Command::new(bin)
        .arg(&state)
        .arg("--seed")
        .arg("1")
        .output()
        .expect("rt should run");

    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert!(!first.stdout.is_empty());
}

#[test]
fn full_url_input_is_accepted() {
    let bin = env!("CARGO_BIN_EXE_rt");
    let url = format!(
        "https://manabeai.github.io/cp-ast-ecosystems/?state={}",
        encoded_state()
    );
    let output = Command::new(bin)
        .arg(url)
        .arg("--seed")
        .arg("1")
        .output()
        .expect("rt should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
}

#[test]
fn input_file_is_accepted() {
    let bin = env!("CARGO_BIN_EXE_rt");
    let path = std::env::temp_dir().join("rt_state.txt");
    fs::write(&path, encoded_state()).expect("state file should be written");
    let output = Command::new(bin)
        .arg(&path)
        .arg("--seed")
        .arg("1")
        .output()
        .expect("rt should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
}
