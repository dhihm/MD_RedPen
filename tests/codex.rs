#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use md_redpen::codex::{CodexClient, CodexRequest};

#[test]
fn uses_noninteractive_read_only_exec_contract() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let script = directory.path().join("codex");
    let args_path = directory.path().join("args.txt");
    let stdin_path = directory.path().join("stdin.txt");
    let env_path = directory.path().join("env.txt");
    fs::write(
        &script,
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' \"$@\" > \"$FAKE_CODEX_ARGS\"\n",
            "cat > \"$FAKE_CODEX_STDIN\"\n",
            "printf '%s|%s' \"$CODEX_API_KEY\" \"$OPENAI_API_KEY\" > \"$FAKE_CODEX_ENV\"\n",
            "printf '가짜 설명'\n",
        ),
    )?;
    let mut permissions = fs::metadata(&script)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions)?;

    let client = CodexClient::at(&script, directory.path())
        .with_timeout(Duration::from_secs(2))
        .with_test_capture(&args_path, &stdin_path, &env_path);
    let request = CodexRequest::explain("RDMA", "RDMA는 빠르다.");

    let result = client.start(&request)?.wait()?;

    assert_eq!(result, "가짜 설명");
    assert_eq!(
        fs::read_to_string(&args_path)?,
        concat!(
            "exec\n",
            "--ephemeral\n",
            "--ignore-user-config\n",
            "--ignore-rules\n",
            "--skip-git-repo-check\n",
            "--sandbox\n",
            "read-only\n",
            "--color\n",
            "never\n",
            "-\n",
        )
    );
    let prompt = fs::read_to_string(&stdin_path)?;
    assert!(prompt.contains("<selection>\nRDMA\n</selection>"));
    assert!(prompt.contains("<context>\nRDMA는 빠르다.\n</context>"));
    assert_eq!(fs::read_to_string(&env_path)?, "|");

    let revision = CodexRequest::revise(
        "느린 문장",
        "느린 문장을 정확하게 고칩니다.",
        "더 구체적으로 바꿔 줘",
    );
    assert_eq!(client.start(&revision)?.wait()?, "가짜 설명");
    let revision_prompt = fs::read_to_string(&stdin_path)?;
    assert!(revision_prompt.contains("<revision_instruction>\n더 구체적으로 바꿔 줘"));
    assert!(revision_prompt.contains("<selection>\n느린 문장\n</selection>"));
    assert!(revision_prompt.contains("replacement"));
    Ok(())
}
