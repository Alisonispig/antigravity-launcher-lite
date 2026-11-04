<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { version as appVersion } from '../package.json';

const language = ref<'zh' | 'en'>(navigator.language.startsWith('zh') ? 'zh' : 'en');
const theme = ref('light');
const t = (zh: string, en: string) => language.value === 'zh' ? zh : en;
const path = ref(navigator.userAgent.includes('Mac') ? '/Applications/Antigravity.app' : '');
const host = ref('127.0.0.1');
const port = ref(7890);
const busy = ref(false);
const message = ref('');
const success = ref(false);
const logs = ref<{time: string; text: string}[]>([]);
const active = ref('home');
const helpTab = ref('steps');
const historyPage = ref(0);
const pageCount = computed(() => Math.max(1, Math.ceil(logs.value.length / 3)));
const visibleLogs = computed(() => logs.value.slice(historyPage.value * 3, historyPage.value * 3 + 3));
const canValidate = computed(() => desktop && !loadingConfig.value && !busy.value && !!path.value);
const desktop = isTauri();
const loadingConfig = ref(desktop);
const writableConfig = ref(false);
const saveState = ref<'idle' | 'saving' | 'saved' | 'failed' | 'invalid' | 'recovered' | 'readError' | 'future'>('idle');
let saveRevision = 0;
interface AppConfig {
 schemaVersion: number; clientPath: string; proxyHost: string; proxyPort: number;
 language: 'zh' | 'en' | null; theme: 'system' | 'light' | 'dark';
}
const saveLabel = computed(() => {
 switch (saveState.value) {
  case 'saving': return t('保存中…', 'Saving…');
  case 'saved': return t('已保存', 'Saved');
  case 'failed': return t('保存失败，请重试', 'Save failed. Retry');
  case 'invalid': return t('请填写有效的主机和端口后保存', 'Enter a valid host and port to save');
  case 'recovered': return t('配置已备份并恢复默认值', 'Config backed up; defaults restored');
  case 'readError': return t('配置读取失败，本次修改不会保存，请重启后重试', 'Could not read settings. Changes will not be saved; restart to retry');
  case 'future': return t('配置来自较新版本，请使用新版启动器', 'Settings require a newer launcher');
  default: return loadingConfig.value ? t('读取配置…', 'Loading settings…') : '';
 }
});
function errorCode(value: unknown) {
 return typeof value === 'object' && value !== null && 'code' in value ? String(value.code) : '';
}
async function saveSettings() {
 if (!desktop || loadingConfig.value || !writableConfig.value) return;
 const revision = ++saveRevision;
 if (!host.value || !Number.isInteger(port.value) || port.value < 1 || port.value > 65535) {
  saveState.value = 'invalid'; return;
 }
 const config: AppConfig = {
  schemaVersion: 1, clientPath: path.value, proxyHost: host.value, proxyPort: port.value,
  language: language.value, theme: theme.value as AppConfig['theme'],
 };
 saveState.value = 'saving';
 try {
  // 不做延时防抖，编辑产生的 IPC 立即交给 Rust 同步命令按顺序落盘。
  await invoke('save_config', {config});
  if (revision === saveRevision) saveState.value = 'saved';
 } catch (error: unknown) {
  if (revision !== saveRevision) return;
  const code = errorCode(error);
  saveState.value = code === 'CONFIG_VERSION_UNSUPPORTED' ? 'future' : code === 'INVALID_PROXY' || code === 'INVALID_PATH' ? 'invalid' : 'failed';
  if (code === 'CONFIG_VERSION_UNSUPPORTED') writableConfig.value = false;
 }
}
onMounted(async () => {
 if (!desktop) return;
 try {
  const loaded = await invoke<{config: AppConfig; recovered: boolean}>('load_config');
  path.value = loaded.config.clientPath;
  host.value = loaded.config.proxyHost;
  port.value = loaded.config.proxyPort;
  if (loaded.config.language) language.value = loaded.config.language;
  theme.value = loaded.config.theme;
  writableConfig.value = true;
  saveState.value = loaded.recovered ? 'recovered' : 'idle';
 } catch (error: unknown) {
  saveState.value = errorCode(error) === 'CONFIG_VERSION_UNSUPPORTED' ? 'future' : 'readError';
 } finally { loadingConfig.value = false; void syncWindowTheme(); }
});
watch([path, host, port, language, theme], () => { void saveSettings(); }, {flush: 'sync'});
async function syncWindowTheme() {
 if (!desktop || loadingConfig.value) return;
 try { await invoke('set_window_theme', {theme: theme.value}); }
 catch { record(t('窗口外观切换失败，请重新选择主题。', 'Could not update the title bar. Please select the theme again.')); }
}
watch(theme, () => { void syncWindowTheme(); }, {flush: 'sync'});
const errors: Record<string, [string, string]> = {
 INVALID_PROXY: ['请输入裸主机名或 IP 和 1–65535 端口；不支持认证。', 'Enter a plain hostname or IP and port 1–65535. Authentication is unsupported.'],
 INVALID_PATH: ['请输入存在的绝对路径。', 'Enter an existing absolute path.'],
 INVALID_APP: ['请选择 Antigravity 客户端；macOS 请选择 .app。', 'Select the Antigravity client; choose its .app on macOS.'],
 NOT_EXECUTABLE: ['目标文件没有可执行权限，请检查安装。', 'The target is not executable. Check its installation.'],
 TCP_FAILED: ['无法连接代理，请检查地址、端口和代理服务。', 'Cannot connect to the proxy. Check its address, port and service.'],
 PROTOCOL_ERROR: ['代理未返回有效 HTTP CONNECT 响应，请使用 HTTP 或混合端口。', 'Invalid HTTP CONNECT response. Use an HTTP or mixed port.'],
 AUTH_REQUIRED: ['代理需要账号密码，请改用无需认证的 HTTP 或混合端口。', 'Use an HTTP or mixed proxy port that does not require a username or password.'],
 CONNECT_REJECTED: ['代理拒绝连接检测目标，请检查代理规则。', 'The proxy rejected the target. Check its rules.'],
 TIMEOUT: ['检测超时，请检查代理或稍后重试。', 'The check timed out. Check the proxy or retry later.'],
 TLS_OR_HTTP_FAILED: ['已连接代理，但无法打开检测页面。请检查节点和代理规则后重试。', 'Connected to the proxy, but the test page could not be reached. Check the connection and proxy rules.'],
 ALREADY_RUNNING: ['Antigravity 已运行，请正常退出后再启动。', 'Antigravity is already running. Quit it normally before launching.'],
 SPAWN_FAILED: ['无法启动客户端，请检查安装和系统权限。', 'Cannot launch the client. Check installation and system permissions.'],
 PROCESS_STATUS_FAILED: ['无法读取子进程状态。', 'Cannot read child process status.'],
 BUSY: ['操作正在进行，请稍候。', 'An operation is in progress. Please wait.'],
 PROBE_FAILED: ['无法初始化网络检测。', 'Cannot initialize the network check.'],
};
const canRun = computed(() => desktop && !loadingConfig.value && !busy.value && !!host.value && Number.isInteger(port.value) && port.value > 0 && port.value <= 65535);
function record(text: string) {
 message.value = text;
 logs.value.unshift({time: new Date().toLocaleTimeString(), text});
 logs.value = logs.value.slice(0, 30);
 historyPage.value = 0;
}
async function run(action: 'validate' | 'probe' | 'launch') {
 if (action === 'validate' ? !canValidate.value : !canRun.value) return;
 busy.value = true; success.value = false;
 message.value = action === 'launch' ? t('校验客户端、检测代理并启动中…', 'Validating client, checking proxy and launching…') : t('正在检测…', 'Checking…');
 const proxy = {host: host.value, port: port.value};
 try {
  if (action === 'validate') {
   await invoke('validate_app', {path: path.value});
   record(t('客户端路径有效。', 'Client path is valid.'));
  } else if (action === 'probe') {
   const r = await invoke<{target: string; status: number; latencyMs: number}>('probe_proxy', {proxy});
   record(t(`连接成功 · ${r.target} · ${r.latencyMs} ms（HTTP ${r.status}）`, `Connected · ${r.target} · ${r.latencyMs} ms (HTTP ${r.status})`));
  } else {
   const r = await invoke<{pid: number; earlyExit: boolean}>('launch_antigravity', {path: path.value, proxy});
   if (r.earlyExit) {
    record(t('Antigravity 启动后很快退出。请检查是否已有实例，或尝试直接打开客户端。', 'Antigravity exited shortly after launch. Check for an existing instance, or try opening the client directly.'));
    return;
   }
   record(t('Antigravity 已启动。请在客户端中确认联网和登录。', 'Antigravity has started. Check connectivity and sign-in inside the client.'));
  }
  success.value = true;
 } catch (error: unknown) {
  const code = typeof error === 'object' && error !== null && 'code' in error ? String(error.code) : '';
  const pair = errors[code];
  record(pair ? t(...pair) : t('操作失败，请确认正在桌面应用中运行。', 'Operation failed. Make sure you are using the desktop app.'));
 } finally { busy.value = false; }
}
</script>

<template>
 <div class="app" :data-theme="theme">
  <header><img src="./assets/app-icon.png" alt="" /><strong>Antigravity <span>Launcher Lite</span></strong></header>
  <nav :aria-label="t('导航', 'Navigation')">
   <button v-for="tab in ['home', 'history', 'settings', 'help']" :key="tab" :class="{selected: active === tab}" @click="active = tab">{{ tab === 'home' ? t('启动', 'Launch') : tab === 'history' ? t('记录', 'History') : tab === 'settings' ? t('设置', 'Settings') : t('帮助', 'Help') }}</button>
  </nav>
  <main>
   <div v-if="active === 'home'" class="home page">
    <h1>{{ t('启动 Antigravity', 'Launch Antigravity') }}</h1>
    <fieldset :disabled="busy || loadingConfig">
     <section class="card">
      <div class="card-heading"><h2>{{ t('客户端', 'Client') }}</h2><span>{{ t('安装路径', 'Installation path') }}</span></div>
      <div class="input-row"><input v-model="path" :aria-label="t('客户端路径', 'Client path')" spellcheck="false" :placeholder="t('输入 Antigravity 的完整路径', 'Full path to Antigravity')"/><button class="secondary" :disabled="!canValidate" @click="run('validate')">{{ t('验证', 'Validate') }}</button></div>
      <p class="hint">{{ t('macOS 选择 .app，Windows 填写 Antigravity.exe 的路径。', 'Use the .app on macOS, or the Antigravity.exe path on Windows.') }}</p>
     </section>
     <section class="card">
      <div class="card-heading"><h2>{{ t('代理', 'Proxy') }}</h2><span>HTTP / Mixed</span></div>
      <div class="proxy-fields"><div><label for="host">{{ t('主机', 'Host') }}</label><input id="host" v-model="host" spellcheck="false" placeholder="127.0.0.1" /></div><div><label for="port">{{ t('端口', 'Port') }}</label><input id="port" v-model.number="port" type="number" min="1" max="65535" /></div><button class="secondary" :disabled="!canRun" @click="run('probe')">{{ t('检测', 'Check') }}</button></div>
      <p class="hint">{{ t('填入代理软件中的 HTTP 或混合端口，无需开启系统代理。', 'Enter the HTTP or mixed port from your proxy app. System proxy is not required.') }}</p>
     </section>
    </fieldset>
    <div class="launch-row"><span>{{ t('仅对本次启动的客户端生效', 'Applies to the client you launch') }}</span><button class="primary" :disabled="!canRun || !path" @click="run('launch')">{{ busy ? t('处理中…', 'Working…') : t('检测并启动', 'Check & launch') }}</button></div>
    <div class="status" role="status" :class="{success, error: !!message && !success && !busy}"><span v-if="busy" class="spinner"></span><p>{{ message || t('填写路径与代理后，点击“检测并启动”。', 'Enter the client path and proxy, then choose Check & launch.') }}</p></div>
   </div>
   <div v-else-if="active === 'history'" class="page history">
    <div class="page-heading"><h1>{{ t('操作记录', 'History') }}</h1><span>{{ t('最近 30 条', 'Last 30 results') }}</span></div>
    <div v-if="!logs.length" class="empty">{{ t('还没有操作记录', 'No activity yet') }}</div>
    <ol v-else class="log-list"><li v-for="(log, i) in visibleLogs" :key="historyPage * 3 + i"><time>{{ log.time }}</time><p>{{ log.text }}</p></li></ol>
    <div class="pagination"><button class="secondary" :disabled="historyPage === 0" @click="historyPage--">{{ t('上一页', 'Previous') }}</button><span>{{ historyPage + 1 }} / {{ pageCount }}</span><button class="secondary" :disabled="historyPage + 1 >= pageCount" @click="historyPage++">{{ t('下一页', 'Next') }}</button></div>
   </div>
   <div v-else-if="active === 'settings'" class="page settings">
    <h1>{{ t('设置', 'Settings') }}</h1>
    <section class="card preferences"><div><label for="lang">{{ t('语言', 'Language') }}</label><select id="lang" v-model="language" :disabled="loadingConfig"><option value="zh">简体中文</option><option value="en">English</option></select></div><div><label for="theme">{{ t('外观', 'Appearance') }}</label><select id="theme" v-model="theme" :disabled="loadingConfig"><option value="system">{{ t('跟随系统', 'System') }}</option><option value="light">{{ t('浅色', 'Light') }}</option><option value="dark">{{ t('深色', 'Dark') }}</option></select></div></section>
    <section class="card"><h2>{{ t('窗口与后台运行', 'Window & background') }}</h2><p>{{ t('关闭窗口后，启动器留在菜单栏或系统托盘。点击图标，选择“打开主窗口”可再次打开；选择“退出”可完全退出。', 'Closing the window keeps the launcher in the menu bar or system tray. Choose Open window to return, or Quit to exit.') }}</p><p>{{ t('macOS 关闭窗口后隐藏 Dock 图标，重新打开窗口时恢复。退出启动器不会关闭已启动的 Antigravity。', 'On macOS, closing the window hides its Dock icon; reopening restores it. Quitting the launcher leaves Antigravity running.') }}</p></section>
   </div>
   <div v-else class="page help">
    <div class="page-heading"><h1>{{ t('帮助', 'Help') }}</h1><div class="segments"><button :class="{chosen: helpTab === 'steps'}" @click="helpTab = 'steps'">{{ t('怎么用', 'Getting started') }}</button><button :class="{chosen: helpTab === 'faq'}" @click="helpTab = 'faq'">{{ t('常见问题', 'Troubleshooting') }}</button></div></div>
    <section v-if="helpTab === 'steps'" class="card instructions">
     <div><b>1</b><div><h2>{{ t('先打开代理软件', 'Start your proxy app') }}</h2><p>{{ t('确认代理已连接，找到 HTTP 或混合端口，例如 7890。主机通常填 127.0.0.1，不需要开启 TUN。', 'Connect your proxy and find its HTTP or mixed port, such as 7890. The host is usually 127.0.0.1. TUN is not needed.') }}</p></div></div>
     <div><b>2</b><div><h2>{{ t('填写客户端路径', 'Enter the client path') }}</h2><p>{{ t('macOS 通常为 /Applications/Antigravity.app；Windows 填写 Antigravity.exe 的完整路径；Linux 填写 antigravity 可执行文件路径。点击“验证”检查。', 'On macOS, use /Applications/Antigravity.app. On Windows, enter the full Antigravity.exe path; on Linux, the antigravity executable. Choose Validate to check it.') }}</p></div></div>
     <div><b>3</b><div><h2>{{ t('检测并启动', 'Check & launch') }}</h2><p>{{ t('先正常退出已打开的 Antigravity，再点击“检测并启动”。检测通过后会打开客户端，随后在客户端中确认联网和登录。', 'Quit any open Antigravity instance normally, then choose Check & launch. Once it opens, confirm connectivity and sign-in inside Antigravity.') }}</p></div></div>
    </section>
    <section v-else class="card faq">
     <div><h2>{{ t('提示无法连接代理？', 'Cannot connect to the proxy?') }}</h2><p>{{ t('确认代理软件正在运行，主机与端口填写正确。请使用 HTTP 或混合端口；仅 SOCKS5 的端口和需要账号密码的代理不能用于此流程。', 'Make sure your proxy is running and the host and port are correct. Use an HTTP or mixed port without a username or password.') }}</p></div>
     <div><h2>{{ t('提示客户端已运行？', 'Antigravity is already running?') }}</h2><p>{{ t('从 Antigravity 菜单中选择退出，再回到启动器重试。仅关闭编辑窗口可能不会退出客户端。', 'Choose Quit from the Antigravity menu, then retry here. Closing an editor window may leave the app running.') }}</p></div>
     <div><h2>{{ t('检测通过，仍无法登录？', 'The check passed, but sign-in fails?') }}</h2><p>{{ t('检测只确认 Google 账号页面可连接。检查代理规则及节点，再查看客户端的错误提示；账号、地区或服务端问题需要分别处理。', 'The check only reaches the Google accounts page. Check your proxy rules and connection, then the client error. Account, region and service errors need separate attention.') }}</p></div>
     <div><h2>{{ t('关闭后怎么找回来？', 'Where is the launcher after closing?') }}</h2><p>{{ t('点击菜单栏或系统托盘中的图标，选择“打开主窗口”。要停止后台运行，选择“退出”。', 'Click its menu bar or tray icon and choose Open window. Choose Quit to stop it running in the background.') }}</p></div>
    </section>
   </div>
  </main>
  <footer><span>{{ t('本应用为独立开发的非官方工具，非 Google 出品。', 'Independent, unofficial tool. Not affiliated with Google.') }}</span><span class="version">v{{ appVersion }}</span><span class="save-feedback" role="status" :class="{'save-error': ['failed', 'invalid', 'readError', 'future'].includes(saveState)}">{{ saveLabel }}<button v-if="saveState === 'failed'" @click="saveSettings">{{ t('重试', 'Retry') }}</button></span></footer>
 </div>
</template>
<style>
:root{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;font-synthesis:none;text-rendering:optimizeLegibility;font-size:13px}*{box-sizing:border-box}html,body,#app{margin:0;width:100%;height:100%;overflow:hidden}button,input,select{font:inherit}button{cursor:pointer}button:disabled{cursor:default;opacity:.4}button:focus-visible{outline:2px solid var(--accent);outline-offset:3px}.app{--bg:#fff;--panel:#fff;--text:#273344;--muted:#7b8695;--border:#e6ebf2;--accent:#527bec;--soft:#eff4ff;height:100%;display:grid;grid-template-rows:40px 38px minmax(0,1fr) 20px;gap:14px;background:var(--bg);color:var(--text);padding:20px 32px 12px}.app[data-theme=dark]{--bg:#1b201d;--panel:#232a25;--text:#e7ece6;--muted:#9ca89f;--border:#343e36;--accent:#a0cbae;--soft:#29372d;color-scheme:dark}@media(prefers-color-scheme:dark){.app[data-theme=system]{--bg:#1b201d;--panel:#232a25;--text:#e7ece6;--muted:#9ca89f;--border:#343e36;--accent:#a0cbae;--soft:#29372d;color-scheme:dark}}header{display:flex;align-items:center;gap:10px}header img{width:36px;height:36px}header strong{font-size:15px;letter-spacing:-.3px}header strong span{font-weight:400;color:var(--muted);margin-left:3px}nav{display:flex;gap:24px;border-bottom:1px solid var(--border)}nav button{background:none;border:0;border-bottom:2px solid transparent;padding:0 1px 9px;color:var(--muted)}nav button.selected{color:var(--text);border-bottom-color:var(--accent)}main{width:100%;max-width:760px;min-height:0;margin:0 auto}.page{height:100%;min-height:0}h1{font-size:20px;font-weight:600;letter-spacing:-.4px;margin:0 0 16px}h2{font-size:13px;font-weight:600;margin:0}p{line-height:1.6;color:var(--muted);margin:8px 0 0;overflow-wrap:anywhere}.home{display:flex;flex-direction:column}.home h1{margin-bottom:12px}fieldset{border:0;padding:0;margin:0;display:grid;gap:12px}.card{border:1px solid var(--border);background:var(--panel);border-radius:9px;padding:16px 18px}.card-heading{display:flex;align-items:center;justify-content:space-between;margin-bottom:10px}.card-heading span{font-size:11px;color:var(--muted)}.input-row{display:flex;gap:10px}.input-row input{flex:1}input,select{min-width:0;width:100%;color:var(--text);background:var(--bg);border:1px solid var(--border);border-radius:6px;height:36px;padding:0 10px}input:focus,select:focus{outline:2px solid var(--accent);outline-offset:1px}label{display:block;color:var(--muted);font-size:11px;margin-bottom:6px}.secondary{color:var(--text);border:1px solid var(--border);background:var(--panel);border-radius:6px;height:36px;padding:0 16px;white-space:nowrap}.proxy-fields{display:grid;grid-template-columns:minmax(0,1fr) 110px auto;gap:10px;align-items:end}.hint{font-size:11px;margin-top:8px}.launch-row{display:flex;align-items:center;justify-content:space-between;gap:16px;padding-top:14px}.launch-row span{font-size:11px;color:var(--muted)}.primary{border:0;background:#527bec;color:#fff;padding:0 28px;height:40px;border-radius:7px;font-weight:500}.status{display:flex;align-items:center;gap:10px;min-height:58px;margin-top:auto;padding-top:8px}.status p{font-size:12px;margin:0}.status.success p{color:var(--accent)}.status.error p{color:#bf6655}.spinner{width:12px;height:12px;flex-shrink:0;border:2px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:spin 1s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}footer{display:flex;align-items:end;justify-content:space-between;gap:12px;color:var(--muted);font-size:10px;border-top:1px solid var(--border);padding-top:7px}.page-heading{display:flex;justify-content:space-between;align-items:center;margin-bottom:14px}.page-heading h1{margin:0}.page-heading>span{font-size:11px;color:var(--muted)}.history{display:flex;flex-direction:column}.empty{margin:auto;color:var(--muted)}.log-list{list-style:none;margin:0;padding:0;display:grid;gap:10px}.log-list li{background:var(--panel);border:1px solid var(--border);border-radius:8px;padding:12px 16px}.log-list time{font-size:10px;color:var(--muted)}.log-list p{color:var(--text);font-size:12px;margin-top:5px}.pagination{display:flex;justify-content:center;align-items:center;gap:18px;margin-top:auto;padding-top:12px}.pagination span{font-size:11px;color:var(--muted)}.pagination button{height:30px;font-size:11px}.preferences{display:grid;grid-template-columns:1fr 1fr;gap:24px;margin-bottom:14px}.settings .card>p{font-size:12px;margin-top:12px}.segments{display:flex;padding:3px;background:var(--border);border-radius:7px;gap:3px}.segments button{background:transparent;color:var(--muted);border:0;border-radius:5px;font-size:11px;padding:6px 10px}.segments .chosen{background:var(--panel);color:var(--text)}.instructions{display:grid;gap:22px;padding:22px}.instructions>div{display:flex;gap:14px}.instructions b{font-size:11px;flex-shrink:0;width:24px;height:24px;display:grid;place-items:center;background:var(--soft);color:var(--accent);border-radius:50%;font-weight:500}.help p{font-size:12px;margin-top:6px}.faq{display:grid;gap:17px;padding:20px}.help h2{line-height:1.5}@media(max-height:660px){.app{padding-top:14px;gap:10px;grid-template-rows:36px 34px minmax(0,1fr) 20px}.card{padding:12px 16px}.home h1{font-size:18px;margin-bottom:10px}.card-heading{margin-bottom:8px}.status{min-height:52px}.instructions{gap:18px;padding:18px}.faq{gap:13px;padding:17px}}
.version{margin-left:auto;white-space:nowrap}.save-feedback{display:flex;align-items:center;gap:8px}.save-error{color:#bf6655}.save-feedback button{border:0;background:transparent;color:inherit;text-decoration:underline;padding:0;font-size:10px}
</style>
