<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface Settings {
  imap_host: string;
  imap_port: number;
  username: string;
  password: string;
  sender_email: string;
  mailbox: string;
}

interface EmailRecord {
  id: number;
  title: string;
  sent_at: string;
  image_filename: string;
}

interface CalendarCell {
  day: number | null;
  dateStr: string;
  records: EmailRecord[];
}

const today = new Date();
const year = ref(today.getFullYear());
const month = ref(today.getMonth() + 1);

const records = ref<EmailRecord[]>([]);
const imageCache = ref<Record<string, string>>({});
const showSettings = ref(false);
const showImage = ref(false);
const selectedImageSrc = ref("");
const selectedImageTitle = ref("");
const statusMessage = ref("");
const isFetching = ref(false);

const settings = ref<Settings>({
  imap_host: "",
  imap_port: 993,
  username: "",
  password: "",
  sender_email: "",
  mailbox: "INBOX",
});

const weekdays = ["日", "月", "火", "水", "木", "金", "土"];

const calendarCells = computed<CalendarCell[]>(() => {
  const firstDay = new Date(year.value, month.value - 1, 1).getDay();
  const daysInMonth = new Date(year.value, month.value, 0).getDate();
  const cells: CalendarCell[] = [];

  for (let i = 0; i < firstDay; i++) {
    cells.push({ day: null, dateStr: "", records: [] });
  }

  for (let d = 1; d <= daysInMonth; d++) {
    const dateStr = `${year.value}-${String(month.value).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
    const dayRecords = records.value.filter((r) => r.sent_at.startsWith(dateStr));
    cells.push({ day: d, dateStr, records: dayRecords });
  }

  return cells;
});

const isToday = (cell: CalendarCell) => {
  if (!cell.day) return false;
  const t = new Date();
  return (
    cell.day === t.getDate() &&
    month.value === t.getMonth() + 1 &&
    year.value === t.getFullYear()
  );
};

async function loadMonthData() {
  try {
    const data = await invoke<EmailRecord[]>("get_month_data", {
      year: year.value,
      month: month.value,
    });
    records.value = data;
    imageCache.value = {};
    for (const record of data) {
      loadThumbnail(record.image_filename);
    }
  } catch (e) {
    statusMessage.value = `データ取得エラー: ${e}`;
  }
}

async function loadThumbnail(filename: string) {
  if (imageCache.value[filename]) return;
  try {
    const dataUrl = await invoke<string>("get_image_data", { filename });
    imageCache.value = { ...imageCache.value, [filename]: dataUrl };
  } catch (_) {
    // 画像が見つからない場合は無視
  }
}

async function openImage(record: EmailRecord) {
  let src = imageCache.value[record.image_filename];
  if (!src) {
    try {
      src = await invoke<string>("get_image_data", {
        filename: record.image_filename,
      });
      imageCache.value = { ...imageCache.value, [record.image_filename]: src };
    } catch (e) {
      statusMessage.value = `画像読み込みエラー: ${e}`;
      return;
    }
  }
  selectedImageSrc.value = src;
  selectedImageTitle.value = record.title;
  showImage.value = true;
}

function prevMonth() {
  if (month.value === 1) {
    month.value = 12;
    year.value--;
  } else {
    month.value--;
  }
}

function nextMonth() {
  if (month.value === 12) {
    month.value = 1;
    year.value++;
  } else {
    month.value++;
  }
}

async function loadSettings() {
  try {
    settings.value = await invoke<Settings>("get_settings");
  } catch (e) {
    statusMessage.value = `設定読み込みエラー: ${e}`;
  }
}

async function saveSettingsAndClose() {
  try {
    await invoke("save_settings", { settings: settings.value });
    showSettings.value = false;
    statusMessage.value = "設定を保存しました";
  } catch (e) {
    statusMessage.value = `設定保存エラー: ${e}`;
  }
}

async function doFetchEmails() {
  isFetching.value = true;
  statusMessage.value = "メールを取得中...";
  try {
    const count = await invoke<number>("fetch_emails");
    statusMessage.value = `${count} 件の画像を取得・保存しました`;
    await loadMonthData();
  } catch (e) {
    statusMessage.value = `取得エラー: ${e}`;
  } finally {
    isFetching.value = false;
  }
}

watch([year, month], loadMonthData);

onMounted(async () => {
  await loadSettings();
  await loadMonthData();
});
</script>

<template>
  <div class="app">
    <!-- Header -->
    <header class="header">
      <div class="header-nav">
        <button class="nav-btn" @click="prevMonth">&#8249;</button>
        <h1 class="month-title">{{ year }}年 {{ month }}月</h1>
        <button class="nav-btn" @click="nextMonth">&#8250;</button>
      </div>
      <div class="header-actions">
        <button
          class="action-btn fetch-btn"
          @click="doFetchEmails"
          :disabled="isFetching"
        >
          {{ isFetching ? "取得中..." : "メール取得" }}
        </button>
        <button class="action-btn settings-btn" @click="showSettings = true">
          設定
        </button>
      </div>
    </header>

    <!-- Status bar -->
    <div v-if="statusMessage" class="status-bar">
      {{ statusMessage }}
      <button class="close-status" @click="statusMessage = ''">✕</button>
    </div>

    <!-- Calendar -->
    <div class="calendar">
      <!-- Weekday headers -->
      <div
        v-for="(wd, i) in weekdays"
        :key="wd"
        class="weekday-header"
        :class="{ sunday: i === 0, saturday: i === 6 }"
      >
        {{ wd }}
      </div>

      <!-- Day cells -->
      <div
        v-for="(cell, idx) in calendarCells"
        :key="idx"
        class="day-cell"
        :class="{
          empty: !cell.day,
          today: isToday(cell),
        }"
      >
        <span v-if="cell.day" class="day-number">{{ cell.day }}</span>
        <div class="thumbnails">
          <img
            v-for="record in cell.records"
            :key="record.id"
            :src="imageCache[record.image_filename]"
            :title="record.title"
            class="thumbnail"
            @click="openImage(record)"
          />
        </div>
      </div>
    </div>

    <!-- Settings modal -->
    <div v-if="showSettings" class="modal-overlay" @click.self="showSettings = false">
      <div class="modal">
        <div class="modal-header">
          <h2>設定</h2>
          <button class="close-btn" @click="showSettings = false">✕</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>IMAPサーバー</label>
            <input v-model="settings.imap_host" placeholder="imap.example.com" />
          </div>
          <div class="form-group">
            <label>ポート</label>
            <input
              v-model.number="settings.imap_port"
              type="number"
              placeholder="993"
            />
          </div>
          <div class="form-group">
            <label>ユーザー名（メールアドレス）</label>
            <input v-model="settings.username" placeholder="you@example.com" />
          </div>
          <div class="form-group">
            <label>パスワード</label>
            <input v-model="settings.password" type="password" placeholder="••••••••" />
          </div>
          <div class="form-group">
            <label>取得対象の送信元アドレス</label>
            <input
              v-model="settings.sender_email"
              placeholder="sender@example.com"
            />
          </div>
          <div class="form-group">
            <label>メールボックス</label>
            <input v-model="settings.mailbox" placeholder="INBOX" />
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn-cancel" @click="showSettings = false">キャンセル</button>
          <button class="btn-save" @click="saveSettingsAndClose">保存</button>
        </div>
      </div>
    </div>

    <!-- Image viewer modal -->
    <div v-if="showImage" class="modal-overlay" @click.self="showImage = false">
      <div class="image-modal">
        <div class="modal-header">
          <button class="back-btn" @click="showImage = false">‹ 戻る</button>
          <h2 class="image-title">{{ selectedImageTitle }}</h2>
          <button class="close-btn" @click="showImage = false">✕</button>
        </div>
        <div class="image-body">
          <img :src="selectedImageSrc" class="full-image" />
        </div>
      </div>
    </div>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  -webkit-tap-highlight-color: transparent;
}

body {
  font-family: "Hiragino Sans", "Hiragino Kaku Gothic ProN", Meiryo, sans-serif;
  background: #f0f2f5;
  color: #1a1a2e;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  overflow: hidden;
}

/* Header */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #1a1a2e;
  color: white;
  padding: 12px 20px;
  flex-shrink: 0;
  gap: 8px;
}

.header-nav {
  display: flex;
  align-items: center;
  gap: 16px;
}

.nav-btn {
  background: rgba(255, 255, 255, 0.15);
  border: none;
  color: white;
  font-size: 22px;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
  touch-action: manipulation;
  flex-shrink: 0;
}

.nav-btn:hover {
  background: rgba(255, 255, 255, 0.3);
}

.month-title {
  font-size: 20px;
  font-weight: 600;
  min-width: 130px;
  text-align: center;
  white-space: nowrap;
}

.header-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.action-btn {
  border: none;
  border-radius: 8px;
  padding: 10px 16px;
  font-size: 14px;
  cursor: pointer;
  font-family: inherit;
  transition: opacity 0.2s;
  touch-action: manipulation;
  min-height: 44px;
  white-space: nowrap;
}

.action-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.fetch-btn {
  background: #4ecdc4;
  color: #1a1a2e;
  font-weight: 600;
}

.settings-btn {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.settings-btn:hover {
  background: rgba(255, 255, 255, 0.3);
}

/* Status bar */
.status-bar {
  background: #ffeaa7;
  color: #2d3436;
  padding: 8px 16px;
  font-size: 13px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}

.close-status {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 16px;
  color: #636e72;
  padding: 4px 8px;
  touch-action: manipulation;
  min-width: 32px;
  min-height: 32px;
}

/* Calendar */
.calendar {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 1px;
  background: #d1d5db;
  flex: 1;
  overflow-y: auto;
  align-content: start;
  -webkit-overflow-scrolling: touch;
}

.weekday-header {
  background: #2d3748;
  color: #e2e8f0;
  text-align: center;
  padding: 8px 0;
  font-size: 13px;
  font-weight: 600;
  position: sticky;
  top: 0;
  z-index: 1;
  user-select: none;
}

.weekday-header.sunday {
  color: #fc8181;
}

.weekday-header.saturday {
  color: #63b3ed;
}

.day-cell {
  background: white;
  min-height: 100px;
  padding: 6px;
  position: relative;
  user-select: none;
}

.day-cell.empty {
  background: #f8f9fa;
}

.day-cell.today {
  background: #ebf8ff;
}

.day-cell.today .day-number {
  background: #3182ce;
  color: white;
  border-radius: 50%;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.day-number {
  font-size: 13px;
  font-weight: 600;
  color: #4a5568;
  display: inline-block;
  margin-bottom: 4px;
}

.thumbnails {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  margin-top: 4px;
}

.thumbnail {
  width: 52px;
  height: 52px;
  object-fit: cover;
  border-radius: 4px;
  cursor: pointer;
  border: 2px solid transparent;
  transition: border-color 0.2s, transform 0.1s;
  touch-action: manipulation;
}

.thumbnail:hover {
  border-color: #3182ce;
  transform: scale(1.05);
}

/* Modals */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal {
  background: white;
  border-radius: 12px;
  width: 480px;
  max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #e2e8f0;
}

.modal-header h2 {
  font-size: 18px;
  color: #1a1a2e;
}

.close-btn {
  background: none;
  border: none;
  font-size: 20px;
  cursor: pointer;
  color: #718096;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background 0.2s;
  touch-action: manipulation;
  min-width: 44px;
  min-height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.close-btn:hover {
  background: #f0f2f5;
}

.modal-body {
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-y: auto;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-group label {
  font-size: 13px;
  color: #4a5568;
  font-weight: 500;
}

.form-group input {
  border: 1px solid #cbd5e0;
  border-radius: 6px;
  padding: 10px 12px;
  font-size: 16px; /* 16px以上でiOSのズームを防ぐ */
  font-family: inherit;
  outline: none;
  transition: border-color 0.2s;
}

.form-group input:focus {
  border-color: #3182ce;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid #e2e8f0;
}

.btn-cancel {
  background: #edf2f7;
  border: none;
  border-radius: 6px;
  padding: 10px 16px;
  font-size: 14px;
  cursor: pointer;
  font-family: inherit;
  color: #4a5568;
  min-height: 44px;
  touch-action: manipulation;
}

.btn-save {
  background: #3182ce;
  border: none;
  border-radius: 6px;
  padding: 10px 20px;
  font-size: 14px;
  cursor: pointer;
  font-family: inherit;
  color: white;
  font-weight: 600;
  min-height: 44px;
  touch-action: manipulation;
}

.btn-save:hover {
  background: #2b6cb0;
}

/* Image viewer */
.image-modal {
  background: #1a1a2e;
  border-radius: 12px;
  width: 90vw;
  max-width: 1000px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
}

.image-modal .modal-header {
  border-bottom-color: #2d3748;
}

.image-title {
  font-size: 16px;
  color: #e2e8f0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.back-btn {
  background: none;
  border: none;
  color: #63b3ed;
  font-size: 16px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  font-family: inherit;
  touch-action: manipulation;
  min-width: 44px;
  min-height: 44px;
  display: none;
  align-items: center;
  flex-shrink: 0;
}

.image-modal .close-btn {
  color: #a0aec0;
}

.image-modal .close-btn:hover {
  background: #2d3748;
}

.image-body {
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: auto;
  flex: 1;
  -webkit-overflow-scrolling: touch;
}

.full-image {
  max-width: 100%;
  max-height: 70vh;
  object-fit: contain;
  border-radius: 4px;
}

/* ===== モバイル対応 (≤ 600px) ===== */
@media (max-width: 600px) {
  /* ヘッダー: 2行に折り返し */
  .header {
    flex-wrap: wrap;
    padding: 8px 12px;
    padding-top: max(8px, env(safe-area-inset-top));
  }

  .header-nav {
    gap: 8px;
    flex: 1;
  }

  .month-title {
    font-size: 16px;
    min-width: 0;
  }

  .nav-btn {
    font-size: 20px;
    width: 40px;
    height: 40px;
  }

  .header-actions {
    width: 100%;
    gap: 6px;
  }

  .action-btn {
    flex: 1;
    font-size: 13px;
    padding: 8px 10px;
    min-height: 40px;
    text-align: center;
  }

  /* カレンダー */
  .weekday-header {
    padding: 5px 0;
    font-size: 11px;
  }

  .day-cell {
    min-height: 60px;
    padding: 3px;
  }

  .day-cell.today .day-number {
    width: 20px;
    height: 20px;
    font-size: 11px;
  }

  .day-number {
    font-size: 11px;
    margin-bottom: 2px;
  }

  .thumbnails {
    gap: 2px;
    margin-top: 2px;
  }

  /* サムネイル: セル幅に合わせて自動調整 */
  .thumbnail {
    width: clamp(28px, calc(100vw / 7 - 10px), 52px);
    height: clamp(28px, calc(100vw / 7 - 10px), 52px);
    border-width: 1px;
  }

  /* モーダル: フルスクリーン */
  .modal-overlay {
    align-items: flex-end;
    padding: 0;
  }

  .modal {
    width: 100%;
    max-width: 100%;
    border-radius: 16px 16px 0 0;
    max-height: 92dvh;
    display: flex;
    flex-direction: column;
    padding-bottom: env(safe-area-inset-bottom);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    max-height: none;
  }

  /* 画像モーダル: フルスクリーン */
  .image-modal {
    width: 100%;
    max-width: 100%;
    height: 100dvh;
    max-height: 100dvh;
    border-radius: 0;
    padding-bottom: env(safe-area-inset-bottom);
  }

  .image-modal .modal-header {
    padding-top: max(16px, env(safe-area-inset-top));
  }

  .back-btn {
    display: flex;
  }

  .image-modal .close-btn {
    display: none;
  }

  .full-image {
    max-height: calc(100dvh - 80px);
  }
}
</style>
