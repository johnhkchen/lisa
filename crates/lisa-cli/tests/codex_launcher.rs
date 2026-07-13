//! Black-box contract for Lisa's interactive Codex argv boundary.

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    #[test]
    fn assignment_path_is_one_uninterpolated_codex_argument() {
        let temp = tempfile::tempdir().unwrap();
        let capture = temp.path().join("captured argv.bin");
        let stub = temp.path().join("codex capture 'stub'.sh");
        fs::write(
            &stub,
            "#!/bin/sh\n: > \"$ARGV_CAPTURE\"\nfor arg in \"$@\"; do\n  printf '%s\\0' \"$arg\" >> \"$ARGV_CAPTURE\"\ndone\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&stub).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&stub, permissions).unwrap();

        let assignment = temp
            .path()
            .join("assignment ' \" $() ; [glob] `tick`\nline.md");
        fs::write(&assignment, "complete attempt-bound assignment\n").unwrap();
        let model = "model ' \" $() ; `tick`";

        let output = Command::new(env!("CARGO_BIN_EXE_lisa"))
            .arg("launch-codex")
            .arg("--codex-bin")
            .arg(&stub)
            .arg("--model")
            .arg(model)
            .arg(&assignment)
            .env("ARGV_CAPTURE", &capture)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "launcher failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let captured = fs::read(&capture).unwrap();
        let mut argv: Vec<&[u8]> = captured.split(|byte| *byte == 0).collect();
        assert_eq!(argv.pop(), Some(&b""[..]), "capture must end with NUL");

        let expected: Vec<Vec<u8>> = vec![
            b"--dangerously-bypass-approvals-and-sandbox".to_vec(),
            b"--dangerously-bypass-hook-trust".to_vec(),
            b"--model".to_vec(),
            model.as_bytes().to_vec(),
            b"--".to_vec(),
            assignment.as_os_str().as_encoded_bytes().to_vec(),
        ];
        assert_eq!(argv, expected);
    }
}
