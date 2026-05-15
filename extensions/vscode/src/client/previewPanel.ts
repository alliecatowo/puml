import * as vscode from 'vscode';

export class PumlPreviewPanel {
  private static panel: vscode.WebviewPanel | undefined;
  private static watcher: vscode.Disposable | undefined;

  static async show(context: vscode.ExtensionContext, document: vscode.TextDocument): Promise<void> {
    if (!PumlPreviewPanel.panel) {
      PumlPreviewPanel.panel = vscode.window.createWebviewPanel('pumlPreview', 'PUML Preview', vscode.ViewColumn.Beside, {
        enableScripts: true,
      });

      PumlPreviewPanel.panel.onDidDispose(() => {
        PumlPreviewPanel.panel = undefined;
        PumlPreviewPanel.watcher?.dispose();
        PumlPreviewPanel.watcher = undefined;
      });

      PumlPreviewPanel.panel.webview.onDidReceiveMessage(async (message) => {
        if (message?.type !== 'replaceDocument' || typeof message.text !== 'string') {
          return;
        }

        const editor = await vscode.window.showTextDocument(document, { preview: false, preserveFocus: true });
        await editor.edit((builder) => {
          const fullRange = new vscode.Range(
            document.positionAt(0),
            document.positionAt(document.getText().length)
          );
          builder.replace(fullRange, message.text);
        });
      });
    }

    PumlPreviewPanel.panel.title = `PUML Studio: ${document.fileName.split('/').pop() ?? 'Untitled'}`;
    PumlPreviewPanel.panel.webview.html = renderWebviewHtml(document.getText());

    PumlPreviewPanel.watcher?.dispose();
    PumlPreviewPanel.watcher = vscode.workspace.onDidChangeTextDocument((event) => {
      if (!PumlPreviewPanel.panel || event.document.uri.toString() !== document.uri.toString()) {
        return;
      }

      PumlPreviewPanel.panel.webview.postMessage({
        type: 'documentUpdated',
        text: event.document.getText(),
      });
    });

    context.subscriptions.push(PumlPreviewPanel.watcher);
    PumlPreviewPanel.panel.reveal(vscode.ViewColumn.Beside);
  }
}

function renderWebviewHtml(source: string): string {
  const boot = JSON.stringify(source);
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>PUML Studio Preview</title>
<style>
  :root { color-scheme: light dark; }
  body { margin: 0; font-family: Inter, Segoe UI, sans-serif; }
  .wrap { display: grid; grid-template-columns: 1fr 1fr; min-height: 100vh; }
  .pane { padding: 12px; box-sizing: border-box; }
  textarea { width: 100%; min-height: 90vh; border-radius: 8px; border: 1px solid #666; padding: 10px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  .toolbar { display:flex; gap:8px; margin-bottom:8px; align-items:center; }
  button { border-radius:6px; border:1px solid #777; padding:4px 10px; cursor:pointer; }
  .canvas { border:1px solid #666; border-radius: 8px; min-height: 90vh; overflow:auto; background: repeating-linear-gradient(45deg,#00000008,#00000008 10px,#00000012 10px,#00000012 20px); }
  svg { min-width: 100%; }
  .hint { font-size: 12px; opacity: .8; }
</style>
</head>
<body>
<div class="wrap">
  <div class="pane">
    <div class="toolbar">
      <button id="apply">Apply to Source</button>
      <span class="hint">Edit here for fast visual iteration (WIP studio mode)</span>
    </div>
    <textarea id="src"></textarea>
  </div>
  <div class="pane">
    <div class="toolbar"><strong>Sequence Preview (scaffold renderer)</strong></div>
    <div class="canvas" id="canvas"></div>
  </div>
</div>
<script>
const vscode = acquireVsCodeApi();
const srcEl = document.getElementById('src');
const canvas = document.getElementById('canvas');
srcEl.value = ${boot};

function parseModel(text){
  const lines = text.split(/\r?\n/);
  const parts = [];
  const seen = new Set();
  const msgs = [];
  for(const raw of lines){
    const line = raw.trim();
    const part = line.match(/^(participant|actor|boundary|control|entity|database|collections|queue)\s+"?([^"\n]+)"?/i);
    if(part){ const name = part[2].trim(); if(!seen.has(name)){ seen.add(name); parts.push(name);} continue; }
    const msg = line.match(/^([^\-:\n]+)\s*(-+>+)\s*([^:\n]+)\s*:\s*(.+)$/);
    if(msg){
      const from = msg[1].trim(), to = msg[3].trim(), label = msg[4].trim();
      if(!seen.has(from)){seen.add(from); parts.push(from);} if(!seen.has(to)){seen.add(to); parts.push(to);} msgs.push({from,to,label});
    }
  }
  return {parts, msgs};
}

function render(text){
  const {parts, msgs} = parseModel(text);
  const lane = 180, margin = 60, top = 40, row = 56;
  const width = Math.max(700, margin*2 + Math.max(parts.length,1)*lane);
  const height = Math.max(240, top + 60 + (msgs.length+1)*row);
  const xFor = (p) => margin + Math.max(parts.indexOf(p),0)*lane + lane/2;

  let out = '<svg xmlns="http://www.w3.org/2000/svg" width="'+width+'" height="'+height+'">';
  out += '<rect width="100%" height="100%" fill="transparent"/>';
  parts.forEach((p,i)=>{ const x = margin + i*lane + 20; out += '<rect x="'+x+'" y="10" width="140" height="26" rx="6" fill="#3b82f6" opacity="0.85" />'; out += '<text x="'+(x+70)+'" y="28" text-anchor="middle" fill="white" font-size="12">'+escapeXml(p)+'</text>'; out += '<line x1="'+(x+70)+'" y1="40" x2="'+(x+70)+'" y2="'+(height-20)+'" stroke="#9ca3af" stroke-dasharray="6,6"/>'; });
  msgs.forEach((m,i)=>{ const y = top + (i+1)*row; const x1 = xFor(m.from), x2 = xFor(m.to); out += '<line x1="'+x1+'" y1="'+y+'" x2="'+x2+'" y2="'+y+'" stroke="#111827" stroke-width="2"/>'; const dir = x1 <= x2 ? 1 : -1; out += '<polygon points="'+(x2)+','+y+' '+(x2-10*dir)+','+(y-5)+' '+(x2-10*dir)+','+(y+5)+'" fill="#111827"/>'; out += '<text x="'+((x1+x2)/2)+'" y="'+(y-8)+'" text-anchor="middle" font-size="12">'+escapeXml(m.label)+'</text>'; });
  if(parts.length===0) out += '<text x="50%" y="50%" text-anchor="middle" fill="#6b7280">Add participants/messages to see a diagram preview.</text>';
  out += '</svg>';
  canvas.innerHTML = out;
}

function escapeXml(s){ return s.replace(/[<>&"']/g, (c)=>({'<':'&lt;','>':'&gt;','&':'&amp;','"':'&quot;',"'":'&#39;'}[c])); }

document.getElementById('apply').addEventListener('click', ()=>vscode.postMessage({type:'replaceDocument', text: srcEl.value}));
srcEl.addEventListener('input', ()=>render(srcEl.value));
window.addEventListener('message', (ev)=>{ if(ev.data?.type==='documentUpdated'){ srcEl.value = ev.data.text; render(srcEl.value);} });
render(srcEl.value);
</script>
</body>
</html>`;
}
