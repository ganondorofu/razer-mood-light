# razer-mood-light

Claude Code が生成中かどうかを、Razer Chroma 対応キーボードの発光で知らせる軽量常駐アプリ(Windows / Rust製)。

- 生成中: 赤で呼吸
- 完了: 緑で呼吸
- 確認待ち(質問・権限プロンプト): 黄色で呼吸
- コンパクト中: シアンで呼吸

タスクトレイに常駐し、HTTPローカルAPI (`127.0.0.1:8765`) 経由で色を切り替える。Claude Code の hooks から呼び出す想定。

## 必要環境

- Windows 10/11
- Razer Synapse 4 + Chroma対応キーボード(Chroma REST APIが有効であること)
- Rust (ビルドする場合)

## ビルド

```sh
cargo build --release
```

`target/release/razer-mood-light.exe` が生成される。

## 使い方

1. exe を実行するとタスクトレイに常駐する(右クリックで終了可能)
2. 以下のエンドポイントにPOSTすると色が切り替わる
   - `POST /generating` — 赤
   - `POST /idle` — 緑
   - `POST /waiting` — 黄色
   - `POST /compacting` — シアン

### Claude Code hooks との連携例

`~/.claude/settings.json` に以下を追加する。

```json
{
  "hooks": {
    "UserPromptSubmit": [
      { "hooks": [{ "type": "command", "command": "curl -s -m 1 -X POST http://127.0.0.1:8765/generating >/dev/null 2>&1 || true" }] }
    ],
    "Stop": [
      { "hooks": [{ "type": "command", "command": "curl -s -m 1 -X POST http://127.0.0.1:8765/idle >/dev/null 2>&1 || true" }] }
    ],
    "Notification": [
      { "hooks": [{ "type": "command", "command": "curl -s -m 1 -X POST http://127.0.0.1:8765/waiting >/dev/null 2>&1 || true" }] }
    ],
    "PreCompact": [
      { "hooks": [{ "type": "command", "command": "curl -s -m 1 -X POST http://127.0.0.1:8765/compacting >/dev/null 2>&1 || true" }] }
    ],
    "PostCompact": [
      { "hooks": [{ "type": "command", "command": "curl -s -m 1 -X POST http://127.0.0.1:8765/generating >/dev/null 2>&1 || true" }] }
    ]
  }
}
```

## 常駐化(自動起動・自動復旧)

タスクスケジューラに登録すると、ログオン時に自動起動し、万一クラッシュしても最大2分以内に自動復帰する(多重起動防止のミューテックスにより、既に起動中なら何もせず終了する)。

```powershell
$action = New-ScheduledTaskAction -Execute "<フルパス>\razer-mood-light.exe"
$logonTrigger = New-ScheduledTaskTrigger -AtLogOn
$watchdogTrigger = New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes 2) -RepetitionDuration (New-TimeSpan -Days 3650)
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Seconds 0) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName "ClaudeMoodLight" -Action $action -Trigger @($logonTrigger, $watchdogTrigger) -Settings $settings
```

## 設定

初回起動時に `%APPDATA%\ClaudeMoodLight\config.json` が自動生成される。色や呼吸速度を変更できる(再ビルド不要)。

```json
{
  "color_generating": "#FF0000",
  "color_idle": "#00FF00",
  "color_waiting": "#FFFF00",
  "color_compacting": "#00FFFF",
  "breath_period_ms": 3000,
  "breath_min": 0.15,
  "breath_step_ms": 100
}
```

- `color_*`: `#RRGGBB` 形式
- `breath_period_ms`: 呼吸1周期の長さ(ミリ秒)
- `breath_min`: 呼吸の最低輝度(0.0-1.0)
- `breath_step_ms`: 明るさ更新の間隔(ミリ秒)。小さいほど滑らかだがリクエスト数が増える

設定を変更したらアプリを再起動する。

## 仕組み

Razer Chroma REST API (`http://localhost:54235/razer/chromasdk`) にキーボードの発光を要求する。CHROMA_BREATHING エフェクトはこの経路では反応しなかったため、CHROMA_CUSTOM エフェクトで明るさを周期的に送り直すことで疑似的な呼吸を実現している。

## ライセンス

MIT。[LICENSE](LICENSE) を参照。
