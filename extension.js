const vscode = require('vscode');
const { spawn } = require('child_process');

const MODELS = {
  'Claude': { cmd: 'claude', args: ['--print', '--dangerously-skip-permissions', '--output-format', 'text'] },
  'Gemini': { cmd: 'gemini', args: ['-y'] },
  'Codex': { cmd: 'codex', args: ['--full-auto', '--quiet'] }
};

async function callAI(model, prompt) {
  const config = MODELS[model];
  if (!config) throw new Error(`Unknown model: ${model}`);

  return new Promise((resolve, reject) => {
    const proc = spawn(config.cmd, config.args, {
      env: { ...process.env, NO_COLOR: '1' }
    });

    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', (data) => { stdout += data.toString(); });
    proc.stderr.on('data', (data) => { stderr += data.toString(); });

    proc.stdin.write(prompt);
    proc.stdin.end();

    proc.on('close', (code) => {
      if (code !== 0) {
        reject(new Error(stderr || `Process exited with code ${code}`));
      } else {
        resolve(stdout.trim());
      }
    });

    proc.on('error', (err) => {
      reject(err);
    });
  });
}

function activate(context) {
  const disposable = vscode.commands.registerCommand('markdown-annotator.addAnnotation', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      vscode.window.showWarningMessage('No active editor');
      return;
    }

    const selection = editor.selection;
    const selectedText = editor.document.getText(selection);

    if (!selectedText) {
      vscode.window.showWarningMessage('텍스트를 선택해주세요');
      return;
    }

    const choice = await vscode.window.showQuickPick(
      ['직접 입력', 'Claude', 'Gemini', 'Codex'],
      { placeHolder: '메모 작성 방법 선택' }
    );

    if (!choice) return;

    let note = '';

    if (choice === '직접 입력') {
      const input = await vscode.window.showInputBox({
        prompt: '메모를 입력하세요',
        placeHolder: '이 텍스트에 대한 설명...'
      });
      if (input === undefined) return;
      note = input;
    } else {
      const defaultPrompt = `다음 텍스트를 한국어로 간단명료하게 1-2문장으로 설명해줘. 설명만 출력하고 다른 말은 하지마: "${selectedText}"`;
      
      const prompt = await vscode.window.showInputBox({
        prompt: `${choice}에게 보낼 프롬프트`,
        value: defaultPrompt
      });
      
      if (prompt === undefined) return;

      await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `${choice}에게 요청 중...`,
        cancellable: false
      }, async () => {
        try {
          note = await callAI(choice, prompt);
        } catch (err) {
          vscode.window.showErrorMessage(`${choice} 호출 실패: ${err.message}`);
          return;
        }
      });

      if (!note) return;

      const confirmNote = await vscode.window.showInputBox({
        prompt: 'AI 응답 확인 (수정 가능)',
        value: note
      });

      if (confirmNote === undefined) return;
      note = confirmNote;
    }

    if (!note) {
      vscode.window.showWarningMessage('메모가 비어있습니다');
      return;
    }

    const escapedNote = note.replace(/"/g, '&quot;').replace(/\n/g, ' ');
    const annotatedText = `<mark data-note="${escapedNote}">${selectedText}</mark>`;

    await editor.edit(editBuilder => {
      editBuilder.replace(selection, annotatedText);
    });

    vscode.window.showInformationMessage('Annotation 추가됨');
  });

  context.subscriptions.push(disposable);
}

function deactivate() {}

module.exports = { activate, deactivate };
