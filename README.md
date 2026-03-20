# Zarigani

メールに添付された画像をカレンダー形式で閲覧するデスクトップアプリ。

## 機能

- 指定した送信元アドレスからのメールを IMAP で取得し、添付画像をローカルに保存
- 月ベースのカレンダー UI で画像を日付ごとに一覧表示
- サムネイルをクリックすると画像を拡大表示
- 取得済み画像の情報を SQLite データベースで管理（重複取得なし）

## 技術スタック

- **フロントエンド**: Vue 3 + TypeScript + Vite
- **バックエンド**: Rust (Tauri 2)
- **データベース**: SQLite (rusqlite)
- **メール**: IMAP over TLS

## セットアップ

### 必要環境

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri の前提条件](https://tauri.app/start/prerequisites/)

### インストール・起動

```bash
npm install
npm run tauri dev
```

## 使い方

### 1. 設定

右上の「設定」ボタンから IMAP 情報を入力して保存する。

| 項目 | 説明 | 例 |
|---|---|---|
| IMAP サーバー | メールサーバーのホスト名 | `imap.gmail.com` |
| ポート | IMAP ポート番号 | `993` |
| ユーザー名 | メールアドレス | `you@example.com` |
| パスワード | メールのパスワード | |
| 取得対象の送信元 | フィルタするメールアドレス | `sender@example.com` |
| メールボックス | 対象フォルダ | `INBOX` |

### 2. メール取得

「メール取得」ボタンをクリックすると、指定した送信元からのメールを取得して添付画像を保存する。

### 3. カレンダー閲覧

`‹` `›` ボタンで月を切り替える。画像が添付されたメールがある日付にはサムネイルが表示される。サムネイルをクリックすると拡大表示される。

## データの保存先

| ファイル | 場所 |
|---|---|
| 設定ファイル | `~/Library/Application Support/jp.geebee.zarigani/settings.json` |
| データベース | `~/Library/Application Support/jp.geebee.zarigani/zarigani.db` |
| 画像ファイル | `~/Library/Application Support/jp.geebee.zarigani/images/` |

## データのリセット

```bash
rm ~/Library/Application\ Support/jp.geebee.zarigani/zarigani.db
rm -rf ~/Library/Application\ Support/jp.geebee.zarigani/images/
```

削除後に「メール取得」を実行すると再取得される。設定は残る。
