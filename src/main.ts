import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './style.css';

type Page = 'home' | 'play' | 'instances' | 'mods' | 'modpacks' | 'worlds' | 'servers' | 'accounts' | 'downloads' | 'settings' | 'console';
type Method = 'microsoft' | 'offline' | 'session';
type Version = { id: string; type: string; url: string };
type Instance = { id: string; name: string; versionId: string; loader?: string; loaderVersion?: string | null };
type ModProject = { projectId: string; title: string; description: string; iconUrl?: string };
type LocalMod = { name: string; enabled: boolean };
type Account = { id: string; name: string; uuid: string; kind: Method; active: boolean };
type Progress = { version: string; done: number; total: number; message: string };
type DevicePrompt = { userCode: string; verificationUri: string; expiresIn: number; message: string };

const root = document.getElementById('app')!;
const previewMode = !('__TAURI_INTERNALS__' in window);
const saved = <T>(key: string, fallback: T): T => {
  try { const raw = localStorage.getItem(`pebble:${key}`); return raw === null ? fallback : JSON.parse(raw) as T; }
  catch { return fallback; }
};
const save = (key: string, value: unknown) => localStorage.setItem(`pebble:${key}`, JSON.stringify(value));
const esc = (value: unknown): string => String(value ?? '').replace(/[&<>"']/g, char =>
  ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]!));

const paths: Record<string, string> = {
  home: '<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1z"/>',
  play: '<path d="m8 5 12 7-12 7z"/>',
  instances: '<path d="m12 2 9 5-9 5-9-5 9-5ZM3 12l9 5 9-5M3 17l9 5 9-5"/>',
  mods: '<path d="M9 3h6v4h3a3 3 0 1 1 0 6h3v7h-6v-3a3 3 0 1 0-6 0v3H3v-7h3a3 3 0 1 0 0-6H3V3z"/>',
  modpacks: '<path d="m12 2 9 5v10l-9 5-9-5V7l9-5ZM3 7l9 5 9-5M12 12v10"/>',
  worlds: '<circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2c-4 5-4 15 0 20M12 2c4 5 4 15 0 20"/>',
  servers: '<rect x="4" y="3" width="16" height="7" rx="1"/><rect x="4" y="14" width="16" height="7" rx="1"/><path d="M8 7h.01M8 18h.01M13 7h4M13 18h4"/>',
  accounts: '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3.1a4 4 0 0 1 0 7.8M22 21v-2a4 4 0 0 0-3-3.87"/><circle cx="9" cy="7" r="4"/>',
  downloads: '<path d="M12 3v12m0 0 4-4m-4 4-4-4M4 17v4h16v-4"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="m19.4 15 .1.1 1.2 1-.9 1.7-1.6-.5a8.8 8.8 0 0 1-2.2 1.3l-.3 1.7h-2l-.4-1.7a8 8 0 0 1-2.5 0l-.4 1.7h-2L8 18.6a8.5 8.5 0 0 1-2.2-1.3l-1.6.5-.9-1.7L4.6 15a8 8 0 0 1 0-2l-1.3-1.1.9-1.7 1.6.5A8.5 8.5 0 0 1 8 9.4l.3-1.7h2l.4 1.7a8 8 0 0 1 2.5 0l.4-1.7h2l.4 1.7a8.8 8.8 0 0 1 2.2 1.3l1.6-.5.9 1.7-1.2 1a8 8 0 0 1 0 2Z" transform="translate(0 -2)"/>',
  console: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="m7 9 3 3-3 3M12 15h5"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/>',
  check: '<path d="m4 12 5 5L20 6"/>',
  refresh: '<path d="M20 7v5h-5M4 17v-5h5M5.7 9A7 7 0 0 1 18 6l2 1M4 17l2 1a7 7 0 0 0 12.3-3"/>',
  trash: '<path d="M4 7h16M10 11v6m4-6v6M6 7l1 14h10l1-14M9 7V4h6v3"/>',
  key: '<circle cx="7" cy="16" r="3"/><path d="m9 14 11-11 2 2-3 3 2 2-3 3-2-2-5 5"/>',
  user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  info: '<circle cx="12" cy="12" r="10"/><path d="M12 11v6M12 7h.01"/>',
  folder: '<path d="M3 6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v10H3z"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="1"/><path d="M16 8V4H4v12h4"/>',
  arrow: '<path d="M5 12h14m-6-6 6 6-6 6"/>',
  shield: '<path d="m12 2 9 4v6c0 5-4 8-9 10-5-2-9-5-9-10V6z"/><path d="m8 12 3 3 5-6"/>',
  sparkles: '<path d="m12 2 2 6 6 2-6 2-2 6-2-6-6-2 6-2zM19 17l1 2 2 1-2 1-1 2-1-2-2-1 2-1z"/>',
  external: '<path d="M13 5h6v6M19 5l-9 9M19 14v5H5V5h5"/>',
  close: '<path d="m5 5 14 14M19 5 5 19"/>',
  list: '<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>',
  menu: '<path d="M4 6h16M4 12h16M4 18h16"/>',
};
function icon(name: string, size = 20): string {
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] || paths.info}</svg>`;
}
function microsoftLogo(): string {
  return '<span class="ms-logo" aria-hidden="true"><i></i><i></i><i></i><i></i></span>';
}
function avatar(name: string, kind: string, size = 'normal'): string {
  // Original pixel-art placeholder; Minecraft textures are not redistributed.
  const hues: Record<string, string> = { microsoft: '#95624b', offline: '#eab674', session: '#35343a' };
  const skin = kind === 'session' ? '#d49b83' : kind === 'offline' ? '#f6bd91' : '#d99c75';
  const hair = hues[kind] || '#96654c';
  const eye = kind === 'session' ? '#c93d3d' : kind === 'offline' ? '#4eb370' : '#8367df';
  return `<span class="pixel-avatar ${size === 'large' ? 'large' : ''}" role="img" aria-label="${esc(name)} avatar"><svg viewBox="0 0 32 32" shape-rendering="crispEdges" aria-hidden="true"><rect width="32" height="32" rx="3" fill="#343343"/><rect x="3" y="5" width="26" height="23" fill="${skin}"/><rect x="3" y="4" width="26" height="9" fill="${hair}"/><rect x="3" y="8" width="5" height="9" fill="${hair}"/><rect x="24" y="8" width="5" height="8" fill="${hair}"/><rect x="8" y="15" width="5" height="4" fill="white"/><rect x="19" y="15" width="5" height="4" fill="white"/><rect x="10" y="15" width="2" height="4" fill="${eye}"/><rect x="20" y="15" width="2" height="4" fill="${eye}"/><rect x="14" y="21" width="4" height="2" fill="#b67765"/></svg></span>`;
}
const label: Record<Page, string> = {
  home: 'Home', play: 'Play', instances: 'Instances', mods: 'Mods', modpacks: 'Modpacks',
  worlds: 'Worlds', servers: 'Servers', accounts: 'Accounts', downloads: 'Downloads', settings: 'Settings', console: 'Console',
};
const navigation: Page[] = ['home', 'play', 'instances', 'mods', 'modpacks', 'worlds', 'servers', 'accounts', 'downloads', 'settings', 'console'];
let page: Page = 'accounts';
let versions: Version[] = [];
let installed: string[] = [];
let instances: Instance[] = [];
let accounts: Account[] = [];
let selectedVersion = saved('version', '');
let selectedInstanceId = saved('instanceId', '');
let selectedAccountId = saved('accountId', '');
let accountSearch = '';
let selectedMethod: Method = 'microsoft';
let showSnapshots = saved('snapshots', false);
let javaPath = saved('java', 'auto');
let memoryMb = saved('ram', 4096);
let clientId = saved('clientId', '');
let loginPrompt: DevicePrompt | null = null;
let loginWaiting = false;
let instanceForm = false;
let busy = false;
let loading = true;
let progress: Progress | null = null;
let logs: string[] = [];
let mods: string[] = [];
let localMods: LocalMod[] = [];
let modResults: ModProject[] = [];
let modSearch = '';
let worlds: string[] = [];
let error = '';
let notice = '';
let status = 'Ready';

const activeAccount = (): Account | undefined => accounts.find(account => account.active);
const selectedAccount = (): Account | undefined => accounts.find(a => a.id === selectedAccountId) || activeAccount() || accounts[0];
const selectedInstance = (): Instance | undefined => instances.find(i => i.id === selectedInstanceId);
const statusChip = (text: string, tone = '') => `<span class="pill ${tone}">${esc(text)}</span>`;
const title = (name: string, desc: string) => `<div class="page-heading"><div><h1>${esc(name)}</h1><p>${esc(desc)}</p></div></div>`;
const button = (text: string, action: string, variant = 'secondary', disabled = false, attrs = '') =>
  `<button class="btn ${variant}" data-action="${action}" ${disabled || busy ? 'disabled' : ''} ${attrs}>${text}</button>`;
function versionOptions(current = selectedVersion): string {
  const filtered = versions.filter(v => showSnapshots || v.type === 'release');
  const ids = [...new Set([...installed, ...filtered.map(v => v.id)])];
  return ids.map(id => `<option value="${esc(id)}" ${id === current ? 'selected' : ''}>${esc(id)}</option>`).join('') || '<option value="">No versions available</option>';
}
function accountCard(account: Account): string {
  return `<button class="account-card ${selectedAccount()?.id === account.id ? 'selected' : ''}" data-action="account-select" data-id="${esc(account.id)}">
    ${avatar(account.name, account.kind)}
    <span class="account-card-text"><strong>${esc(account.name)}</strong><small>${account.kind === 'microsoft' ? microsoftLogo() : icon(account.kind === 'offline' ? 'user' : 'key', 14)} ${esc(kindLabel(account.kind))}</small></span>
    ${account.active ? statusChip('Active', 'green') : ''}
    <span class="account-more">•••</span>
  </button>`;
}
function kindLabel(kind: Method): string {
  return kind === 'microsoft' ? 'Microsoft Account' : kind === 'offline' ? 'Offline Account' : 'Session (Imported Token)';
}
function accountsView(): string {
  const filtered = accounts.filter(a => a.name.toLowerCase().includes(accountSearch.toLowerCase()));
  const current = selectedAccount();
  return `<div class="hero accounts-hero"><div class="hero-content"><div class="headline-icon">${icon('accounts', 38)}</div><div><h1>Accounts</h1><p>Manage your Minecraft accounts, add new ones, and switch between them.</p></div></div></div>
    <div class="accounts-columns">
      <section class="surface accounts-list">
        <div class="surface-head"><h2>Your Accounts</h2>${statusChip(`${accounts.length} account${accounts.length === 1 ? '' : 's'}`)}
          <span class="grow"></span><label class="search-field">${icon('search', 17)}<input id="accountSearch" placeholder="Search accounts…" value="${esc(accountSearch)}" aria-label="Search accounts"/></label>
        </div>
        <div class="account-cards">${filtered.map(accountCard).join('') || `<div class="empty-state">${icon('user', 26)}<strong>${accounts.length ? 'No matching accounts' : 'No accounts yet'}</strong><p>Choose an account method on the right to get started.</p></div>`}</div>
        <p class="muted-footnote">${icon('shield', 15)} Account credentials are stored in your operating system's credential vault.</p>
      </section>
      <div class="accounts-right">
        <section class="surface add-card"><div class="surface-title"><span class="purple-icon">${icon('plus', 23)}</span><div><h2>Add Account</h2><p>Choose a method to add a Minecraft account.</p></div></div>
          <div class="method-grid">
            <button class="method ${selectedMethod === 'microsoft' ? 'selected' : ''}" data-action="method" data-method="microsoft">${microsoftLogo()}<strong>Microsoft</strong><small>Sign in with your Minecraft account</small></button>
            <button class="method ${selectedMethod === 'offline' ? 'selected' : ''}" data-action="method" data-method="offline">${icon('user', 27)}<strong>Offline</strong><small>Create a local offline profile</small></button>
            <button class="method ${selectedMethod === 'session' ? 'selected' : ''}" data-action="method" data-method="session">${icon('key', 27)}<strong>Session Token</strong><small>Import your existing Minecraft session</small></button>
          </div>
          ${methodForm()}
        </section>
        <section class="surface selected-account"><div class="surface-title compact"><span class="subtle-icon">${icon('info', 22)}</span><h2>Selected Account</h2></div>
          ${current ? `<div class="selected-profile">${avatar(current.name, current.kind, 'large')}<div><h3>${esc(current.name)}</h3><span class="small-kind">${current.kind === 'microsoft' ? microsoftLogo() : icon(current.kind === 'session' ? 'key' : 'user', 16)} ${esc(kindLabel(current.kind))}</span></div></div>
            <div class="detail-line"><span>UUID</span><code>${esc(current.uuid)}</code>${button(icon('copy', 15), 'copy-uuid', 'icon-only', false, `aria-label="Copy UUID"`)}</div>
            <div class="detail-line"><span>Type</span><code>${esc(kindLabel(current.kind))}</code></div>
            <div class="detail-line"><span>Status</span><strong class="${current.active ? 'status-green' : 'status-muted'}">${current.active ? 'Active for launch' : 'Not active'}</strong></div>
            ${current.kind === 'offline' ? '<p class="field-help">Offline profiles do not verify Minecraft ownership or grant online-mode server access.</p>' : current.kind === 'session' ? '<p class="field-help">Session tokens can expire and cannot be refreshed automatically. Re-import if validation fails.</p>' : ''}
            <div class="selected-actions">
              ${button(`${icon('check', 16)} Set Active`, 'activate-account', 'secondary', current.active)}
              ${button(`${icon('refresh', 16)} Validate`, 'validate-account')}
              ${button(`${icon('trash', 16)} Remove`, 'remove-account', 'danger')}
            </div>` : `<div class="empty-state short">${icon('user', 30)}<p>Select an account to see its details.</p></div>`}
        </section>
      </div>
    </div>`;
}
function methodForm(): string {
  if (selectedMethod === 'microsoft') {
    return `<div class="method-form"><p>Secure sign-in opens the official Microsoft device authorization flow. Configure your application's client ID in Settings.</p>
      ${loginPrompt ? `<div class="device-code"><small>Open <b>${esc(loginPrompt.verificationUri)}</b> and enter this code:</small><strong>${esc(loginPrompt.userCode)}</strong><div>${button(`${icon('copy', 15)} Copy code`, 'copy-device-code', 'secondary')}${statusChip(loginWaiting ? 'Waiting for Microsoft' : 'Ready')}</div></div>` : button(`${microsoftLogo()} Sign in with Microsoft`, 'login', 'primary', !clientId || loginWaiting)}
      ${!clientId ? '<span class="field-help">Enter your Microsoft application ID under Settings first.</span>' : ''}</div>`;
  }
  if (selectedMethod === 'offline') {
    return `<form id="offlineForm" class="method-form"><label for="offlineUsername">Minecraft username</label><div class="inline-form"><input id="offlineUsername" name="username" minlength="3" maxlength="16" pattern="[A-Za-z0-9_]{3,16}" required placeholder="ExamplePlayer" autocomplete="off"/>${button(`${icon('plus', 16)} Add`, 'add-offline', 'primary')}</div><p>For local/offline play only. This does not grant a paid Minecraft license.</p></form>`;
  }
  return `<form id="sessionForm" class="method-form"><label for="sessionToken">Minecraft access token</label><div class="inline-form"><input id="sessionToken" name="token" type="password" autocomplete="off" spellcheck="false" placeholder="Paste your own access token" aria-label="Minecraft session token" required/>${button(`${icon('key', 16)} Import`, 'import-session', 'primary')}</div><p>The token is validated against Minecraft's official profile endpoint and stored only in your OS credential vault. Never share it with anyone.</p></form>`;
}
function homeView(): string {
  const active = activeAccount();
  return `<div class="hero home-hero"><div class="hero-content"><div class="headline-icon">${icon('sparkles', 38)}</div><div><h1>Welcome to Pebble</h1><p>Everything Minecraft, in one lightweight launcher.</p></div></div></div>
  <div class="metrics"><div class="surface metric"><span>Active account</span><strong>${esc(active?.name || 'No account')}</strong><small>${active ? esc(kindLabel(active.kind)) : 'Add an account to play'}</small></div>
  <div class="surface metric"><span>Installed versions</span><strong>${installed.length}</strong><small>Cached game versions</small></div>
  <div class="surface metric"><span>Instances</span><strong>${instances.length}</strong><small>Isolated game folders</small></div></div>
  <section class="surface quick-launch"><div><h2>Quick Launch</h2><p>${esc(selectedVersion || 'Choose a version')} · ${esc(selectedInstance()?.name || 'Default installation')}</p></div>
  ${button(`${icon('play', 18)} Play Minecraft`, 'launch-default', 'primary', !active || !selectedVersion)}</section>
  <div class="split-info"><section class="surface"><h2>Available now</h2><p>Vanilla version downloads, Java launching, Microsoft accounts, offline profiles, secure session import, separate instances, and game logs.</p></section>
  <section class="surface"><h2>Mod management</h2><p>Fabric and Quilt profiles, experimental Forge and NeoForge installers, and compatible Modrinth downloads are available for testing. Modpacks and server tools remain planned.</p></section></div>`;
}
function playView(): string {
  const ready = installed.includes(selectedVersion);
  return `${title('Play Minecraft', 'Select a version, verify its files, and start the game.')}
  <section class="surface primary-content"><div class="panel-row"><div><h2>Minecraft Java Edition</h2><p>Mojang downloads · Vanilla and installed mod-loader instances</p></div>${statusChip(ready ? 'Installed' : 'Not installed', ready ? 'green' : '')}</div>
    <div class="form-two"><label>Game version<select id="versionPicker" ${busy ? 'disabled' : ''}>${versionOptions()}</select></label><label>Launch instance<select id="playInstance" ${busy ? 'disabled' : ''}>
      <option value="">Default game folder</option>${instances.filter(i => i.versionId === selectedVersion).map(i => `<option value="${esc(i.id)}" ${i.id === selectedInstanceId ? 'selected' : ''}>${esc(i.name)}</option>`).join('')}</select></label></div>
    <label class="checkline"><input id="showSnapshots" type="checkbox" ${showSnapshots ? 'checked' : ''}/> Show snapshots</label>
    ${progressMarkup()}
    <div class="action-row">${button(`${icon('download', 17)} ${ready ? 'Verify / repair' : 'Install version'}`, 'install', 'secondary', !selectedVersion)}${button(`${icon('play', 18)} Play`, 'launch-default', 'primary', !selectedVersion || !activeAccount())}</div>
    <p class="field-help">Minecraft Java requires an appropriate Java runtime. Pebble detects Java from Minecraft metadata and can install a compatible Java runtime automatically.</p>
  </section>`;
}
function instancesView(): string {
  return `${title('Instances', 'Separate worlds, settings and mods in independent game directories.')}
  <div class="panel-row instances-toolbar"><div><h2>My Instances</h2><p>${instances.length} configured</p></div>${button(`${icon('plus', 16)} New Instance`, 'toggle-instance-form', 'primary')}</div>
  ${instanceForm ? `<form id="newInstanceForm" class="surface new-instance-form"><div class="form-two"><label>Instance name<input id="newInstanceName" name="name" placeholder="Survival" maxlength="64" required/></label><label>Minecraft version<select id="newInstanceVersion" required>${versionOptions()}</select></label></div><div class="action-row">${button('Cancel', 'toggle-instance-form')}${button(`${icon('plus', 16)} Create instance`, 'create-instance', 'primary', !selectedVersion)}</div></form>` : ''}
  <div class="instance-grid">${instances.map(i => `<section class="surface instance-item"><div class="instance-tile">${icon('modpacks', 25)}</div><div class="instance-info"><h3>${esc(i.name)}</h3><p>${esc(i.loader || 'vanilla')} ${esc(i.loaderVersion || '')} · ${esc(i.versionId)}</p></div>${statusChip(installed.includes(i.versionId) ? 'Installed' : 'Not installed', installed.includes(i.versionId) ? 'green' : '')}
    <div class="instance-loader-config"><select id="loader-${esc(i.id)}" aria-label="Mod loader for ${esc(i.name)}">
      ${['vanilla','fabric','quilt','forge','neoforge'].map(l => `<option value="${l}" ${(i.loader || 'vanilla') === l ? 'selected' : ''}>${l}</option>`).join('')}
      </select><input id="loader-build-${esc(i.id)}" value="${esc(i.loaderVersion || '')}" placeholder="Loader build (auto for Fabric/Quilt)" aria-label="Loader build" />
      ${button('Install Loader', 'install-loader', 'secondary', false, `data-id="${esc(i.id)}"`)}</div>
    <div class="instance-actions">${button(`${icon('play', 17)} Play`, 'launch-instance', 'primary', !activeAccount(), `data-id="${esc(i.id)}"`)}${button(`${icon('folder', 17)} Files`, 'open-instance', 'secondary', false, `data-id="${esc(i.id)}"`)}${button(`${icon('trash', 16)}`, 'remove-instance', 'danger', false, `aria-label="Remove instance" data-id="${esc(i.id)}"`)}</div></section>`).join('') || '<div class="surface empty-state">No instances yet. Create one to keep saves and mods separate.</div>'}</div>`;
}
function modsView(): string {
  const inst = selectedInstance();
  return `${title('Mods', 'Browse compatible Modrinth mods and manage instance JAR files.')}
  <section class="surface primary-content"><div class="panel-row"><h2>Mod manager</h2>${instancePicker()}</div>
  ${inst ? `<p class="field-help">${esc(inst.name)} · ${esc(inst.versionId)} · ${esc(inst.loader || 'vanilla')}. Install and configure a loader under Instances first.</p>
    <div class="inline-form"><input id="modSearch" placeholder="Search Modrinth…" maxlength="90" value="${esc(modSearch)}" aria-label="Search Modrinth mods"/>
    ${button(`${icon('search', 15)} Search`, 'mod-search', 'primary', (inst.loader || 'vanilla') === 'vanilla')}</div>
    <div class="file-list">${modResults.map(m => `<div class="file-item"><div class="grow"><strong>${esc(m.title)}</strong><p class="field-help">${esc(m.description.slice(0, 140))}</p></div>${button('Install', 'mod-install', 'secondary', false, `data-id="${esc(m.projectId)}"`)}</div>`).join('') || '<div class="empty-state">Search for mods compatible with your game version and loader.</div>'}</div>
    <h3 class="section-subheading">Installed JARs</h3>
    <div class="file-list">${localMods.map(m => `<div class="file-item">${icon('mods', 17)}<span class="grow">${esc(m.name)}</span>${statusChip(m.enabled ? 'Enabled' : 'Disabled', m.enabled ? 'green' : '')}
    ${button(m.enabled ? 'Disable' : 'Enable', 'mod-toggle', 'secondary', false, `data-id="${esc(m.name)}" data-enabled="${m.enabled ? 'false' : 'true'}"`)}${button('Remove', 'mod-remove', 'danger', false, `data-id="${esc(m.name)}"`)}</div>`).join('') || '<div class="empty-state">No installed mods yet.</div>'}</div>
    <div class="action-row">${button(`${icon('folder', 17)} Open Mods Folder`, 'open-mods')}</div>` : '<p>Create or select an instance to manage mods.</p>'}
  </section>`;
}
function worldsView(): string {
  return `${title('Worlds', 'Inspect saved worlds for a selected instance.')}
  <section class="surface primary-content"><div class="panel-row"><h2>Saved worlds</h2>${instancePicker()}</div>
  ${selectedInstance() ? `<div class="file-list">${worlds.map(w => `<div class="file-item">${icon('worlds', 18)}<span>${esc(w)}</span></div>`).join('') || '<div class="empty-state">No saved worlds in this instance.</div>'}</div>
  <div class="action-row">${button(`${icon('folder', 17)} Open Saves Folder`, 'open-saves', 'secondary')}</div>` : '<p>Select or create an instance first.</p>'}</section>`;
}
function instancePicker(): string {
  return `<select id="activeInstance" class="compact-select"><option value="">Select instance…</option>${instances.map(i => `<option value="${esc(i.id)}" ${i.id === selectedInstanceId ? 'selected' : ''}>${esc(i.name)}</option>`).join('')}</select>`;
}
function futureView(section: 'modpacks' | 'servers'): string {
  const packs = section === 'modpacks';
  return `${title(packs ? 'Modpacks' : 'Servers', packs ? 'Modpack integration roadmap' : 'Server management roadmap')}
  <section class="surface feature-coming"><span class="feature-glyph">${icon(packs ? 'modpacks' : 'servers', 40)}</span><h2>${packs ? 'Modpack engine — upcoming' : 'Server manager — upcoming'}</h2>
  <p>${packs ? 'Planned support for Modrinth, CurseForge, FTB, ATLauncher, dependencies and pack import/export.' : 'Planned tools for local Vanilla, Fabric, Forge and Paper servers, configuration, logs and backups.'}</p>
  ${statusChip('Not implemented in this source', 'warning')}
  <p class="field-help">This is an architecture placeholder, not an operational installer. No action is required now.</p></section>`;
}
function downloadsView(): string {
  return `${title('Downloads', 'Minecraft installation activity and local version cache.')}
  <section class="surface primary-content"><h2>Current activity</h2>${progressMarkup() || '<div class="empty-state">No active download.</div>'}
  <h3 class="section-subheading">Installed Minecraft versions</h3><div class="file-list">${installed.map(id => `<div class="file-item">${icon('check', 17)}<span>${esc(id)}</span>${statusChip('Cached', 'green')}</div>`).join('') || '<p class="muted">None installed.</p>'}</div></section>`;
}
function progressMarkup(): string {
  if (!progress) return '';
  const pct = progress.total ? Math.min(100, Math.round(progress.done * 100 / progress.total)) : 0;
  return `<div class="download-progress"><div><span>${esc(progress.message)} · ${esc(progress.version)}</span><strong>${pct}%</strong></div><div class="progress-track"><div style="width:${pct}%"></div></div><small>${progress.done} of ${progress.total} files</small></div>`;
}
function settingsView(): string {
  return `${title('Settings', 'Launcher configuration and Minecraft launch options.')}
  <section class="surface primary-content settings-form"><div class="panel-row"><h2>Java & Game</h2>${statusChip('Local settings')}</div>
    <label>Java runtime (auto or custom executable)<input id="javaPath" value="${esc(javaPath)}" spellcheck="false" placeholder="auto"/></label>
    <p class="field-help">Use <code>auto</code> to detect and install the version specified by Minecraft metadata (Java 21 for 1.21.x; newer versions may require Java 25). A custom path is optional.</p>
    <label>Maximum memory (MB)<input id="memoryMb" type="number" value="${memoryMb}" min="1024" max="32768" step="256"/></label>
    <div class="panel-row section-heading"><h2>Authentication</h2>${icon('shield', 20)}</div>
    <label>Microsoft application (client) ID<input id="clientId" value="${esc(clientId)}" spellcheck="false" placeholder="Public Azure app client ID"/></label>
    <p class="field-help">Use a public-client registration authorized to use Minecraft services. No Microsoft passwords are collected.</p>
    <label class="checkline"><input type="checkbox" id="settingsSnapshots" ${showSnapshots ? 'checked' : ''}/> Include Minecraft snapshots in version selection</label>
    <div class="action-row">${button(`${icon('check', 17)} Save Settings`, 'save-settings', 'primary')}</div></section>`;
}
function consoleView(): string {
  return `${title('Game Console', 'Live output from Minecraft Java Edition.')}
    <section class="surface primary-content"><div class="panel-row"><h2>Game output</h2>${button('Clear', 'clear-logs')}</div>
      <pre class="game-console" id="gameConsole">${esc(logs.slice(-250).join('\n') || 'No game logs yet. Launch Minecraft to see process output.')}</pre></section>`;
}
function render(): void {
  const main: Record<Page, () => string> = {
    home: homeView, play: playView, instances: instancesView, mods: modsView, modpacks: () => futureView('modpacks'),
    worlds: worldsView, servers: () => futureView('servers'), accounts: accountsView, downloads: downloadsView,
    settings: settingsView, console: consoleView,
  };
  root.innerHTML = `<div class="app-shell"><aside class="sidebar"><div class="brand"><div class="brand-logo"><span class="logo-cube"></span></div><div><strong>Pebble Launcher</strong><small>v0.3.0-dev</small></div></div>
    <nav class="nav-list" aria-label="Main navigation">${navigation.map(p => `<button data-action="navigate" data-page="${p}" class="nav-item ${page === p ? 'active' : ''}" aria-current="${page === p ? 'page' : 'false'}">${icon(p, 21)}<span>${label[p]}</span></button>`).join('')}</nav>
    <div class="sidebar-bottom"><span class="block-emoji">▥</span><span>Minecraft<br/><b>${previewMode ? 'UI preview' : 'Ready'}</b></span><span class="status-dot"></span></div></aside>
    <main class="main-area"><div class="topbar"><span>${icon('modpacks', 16)} Minecraft Java Edition</span><span>${previewMode ? statusChip('Browser preview', 'warning') : statusChip('Development build')}</span></div>
    <div class="page-area">${loading ? '<div class="empty-state">Loading launcher data…</div>' : main[page]()}</div></main>
    <div class="statusbar"><span class="status-dot"></span>${esc(status)}<span class="grow"></span>${icon('user', 15)} ${esc(activeAccount()?.name || 'No active account')}</div>
    ${error ? `<div class="toast error" role="alert">${icon('info', 19)}<span>${esc(error)}</span><button data-action="dismiss" aria-label="Dismiss">${icon('close', 16)}</button></div>` : ''}
    ${notice && !error ? `<div class="toast success" role="status">${icon('check', 18)}<span>${esc(notice)}</span><button data-action="dismiss" aria-label="Dismiss">${icon('close', 16)}</button></div>` : ''}
  </div>`;
  const consoleEl = document.getElementById('gameConsole');
  if (consoleEl) consoleEl.scrollTop = consoleEl.scrollHeight;
}
function fail(e: unknown): void {
  error = String(e instanceof Error ? e.message : e);
  notice = ''; status = 'Action failed';
}
function requireBackend(): void {
  if (previewMode) throw new Error('This is the web UI preview. Run the Tauri application for account and Minecraft operations.');
}
async function syncAccounts(): Promise<void> {
  accounts = await invoke<Account[]>('list_accounts');
  if (!accounts.some(a => a.id === selectedAccountId)) selectedAccountId = activeAccount()?.id || accounts[0]?.id || '';
  save('accountId', selectedAccountId);
}
async function syncInstances(): Promise<void> {
  instances = await invoke<Instance[]>('list_instances');
  if (!instances.some(i => i.id === selectedInstanceId)) selectedInstanceId = instances[0]?.id || '';
  save('instanceId', selectedInstanceId);
}
async function syncFiles(): Promise<void> {
  if (!selectedInstanceId || previewMode) { mods = []; localMods = []; worlds = []; return; }
  const [m, w] = await Promise.allSettled([
    invoke<string[]>('list_instance_mods', { instanceId: selectedInstanceId }),
    invoke<string[]>('list_instance_worlds', { instanceId: selectedInstanceId }),
  ]);
  mods = m.status === 'fulfilled' ? m.value : [];
  worlds = w.status === 'fulfilled' ? w.value : [];
  try { localMods = await invoke<LocalMod[]>('list_local_mods', { instanceId: selectedInstanceId }); } catch { localMods = []; }
}
async function installVersion(id: string): Promise<void> {
  if (!installed.includes(id)) {
    progress = null;
    status = `Installing Minecraft ${id}`;
    render();
    await invoke('install_version', { versionId: id });
    installed = [...new Set([...installed, id])];
  }
}
async function playVersion(versionId: string, instanceId?: string): Promise<void> {
  if (!versionId) throw new Error('Select a Minecraft version first');
  if (!activeAccount()) throw new Error('Select an active account in Accounts first');
  await installVersion(versionId);
  status = `Launching Minecraft ${versionId}`; render();
  await invoke('launch_game', { versionId, instanceId: instanceId || null, javaPath, memoryMb });
  notice = `Started Minecraft ${versionId}`;
  status = 'Minecraft launched';
}
function formValue(id: string): string { return (document.getElementById(id) as HTMLInputElement | HTMLSelectElement | null)?.value?.trim() || ''; }
function setMethod(method: Method): void { selectedMethod = method; loginPrompt = null; error = ''; notice = ''; render(); }
async function action(name: string, element: HTMLElement): Promise<void> {
  if (busy && !['dismiss', 'copy-device-code', 'copy-uuid'].includes(name)) return;
  error = ''; notice = '';
  if (name === 'dismiss') { error = ''; notice = ''; render(); return; }
  if (name === 'navigate') {
    page = element.dataset.page as Page; render();
    if (page === 'mods' || page === 'worlds') { await syncFiles(); render(); }
    return;
  }
  if (name === 'method') { setMethod(element.dataset.method as Method); return; }
  if (name === 'account-select') { selectedAccountId = element.dataset.id || ''; save('accountId', selectedAccountId); render(); return; }
  if (name === 'toggle-instance-form') { instanceForm = !instanceForm; render(); return; }
  if (name === 'copy-device-code' || name === 'copy-uuid') {
    const value = name === 'copy-device-code' ? loginPrompt?.userCode : selectedAccount()?.uuid;
    if (value) {
      try { await navigator.clipboard.writeText(value); notice = 'Copied to clipboard'; }
      catch { error = 'Clipboard unavailable. Copy the text manually.'; }
    }
    render(); return;
  }
  if (name === 'clear-logs') { logs = []; render(); return; }
  // Gather secret and form values BEFORE re-rendering, which removes input fields.
  const inputToken = name === 'import-session' ? formValue('sessionToken') : '';
  const inputUsername = name === 'add-offline' ? formValue('offlineUsername') : '';
  const inputInstanceName = name === 'create-instance' ? formValue('newInstanceName') : '';
  const inputInstanceVersion = name === 'create-instance' ? formValue('newInstanceVersion') : '';
  const newJava = name === 'save-settings' ? formValue('javaPath') : '';
  const searchText = name === 'mod-search' ? formValue('modSearch') : '';
  const loaderKind = name === 'install-loader' ? formValue(`loader-${element.dataset.id || ''}`) : '';
  const loaderBuild = name === 'install-loader' ? formValue(`loader-build-${element.dataset.id || ''}`) : '';
  const newMemory = name === 'save-settings' ? formValue('memoryMb') : '';
  const newClientId = name === 'save-settings' ? formValue('clientId') : '';
  const newSnapshots = name === 'save-settings' ? ((document.getElementById('settingsSnapshots') as HTMLInputElement | null)?.checked ?? showSnapshots) : showSnapshots;
  const targetId = element.dataset.id || '';
  const current = selectedAccount();
  try {
    requireBackend(); busy = true; render();
    switch (name) {
      case 'login': {
        if (!clientId) throw new Error('Set your Microsoft application ID in Settings');
        loginPrompt = await invoke<DevicePrompt>('start_login', { clientId });
        loginWaiting = true;
        busy = false; status = 'Waiting for Microsoft authorization'; render();
        void finishLogin();
        return;
      }
      case 'add-offline':
        if (!inputUsername) throw new Error('Enter an offline username');
        accounts = await invoke<Account[]>('add_offline_account', { username: inputUsername });
        selectedAccountId = activeAccount()?.id || ''; notice = 'Offline profile added'; break;
      case 'import-session':
        if (!inputToken) throw new Error('Enter your own Minecraft access token');
        accounts = await invoke<Account[]>('import_session', { token: inputToken });
        selectedAccountId = activeAccount()?.id || ''; notice = 'Session validated and securely imported'; break;
      case 'activate-account':
        if (!current) return;
        accounts = await invoke<Account[]>('select_account', { accountId: current.id }); notice = `${current.name} is now active`; break;
      case 'validate-account':
        if (!current) return;
        await invoke('validate_account', { accountId: current.id }); notice = 'Account is valid'; break;
      case 'remove-account':
        if (!current) return;
        if (!window.confirm(`Remove ${current.name} from this launcher? Saved credentials will be deleted.`)) return;
        accounts = await invoke<Account[]>('remove_account', { accountId: current.id });
        selectedAccountId = activeAccount()?.id || accounts[0]?.id || ''; notice = 'Account removed'; break;
      case 'install':
        if (!selectedVersion) throw new Error('Select a Minecraft version');
        progress = null; status = 'Verifying Minecraft files'; render();
        await invoke('install_version', { versionId: selectedVersion });
        installed = [...new Set([...installed, selectedVersion])]; notice = `${selectedVersion} installed and verified`; break;
      case 'launch-default':
        await playVersion(selectedVersion, selectedInstance()?.versionId === selectedVersion ? selectedInstanceId : undefined); break;
      case 'launch-instance': {
        const inst = instances.find(i => i.id === targetId);
        if (!inst) throw new Error('Instance not found');
        selectedInstanceId = inst.id; selectedVersion = inst.versionId;
        await playVersion(inst.versionId, inst.id); break;
      }
      case 'create-instance': {
        if (!inputInstanceName || !inputInstanceVersion) throw new Error('Enter an instance name and version');
        const created = await invoke<Instance>('create_instance', { name: inputInstanceName, versionId: inputInstanceVersion });
        await syncInstances(); selectedInstanceId = created.id; selectedVersion = created.versionId;
        instanceForm = false; notice = `${created.name} created`; break;
      }
      case 'remove-instance': {
        const inst = instances.find(i => i.id === targetId);
        if (!inst) throw new Error('Instance not found');
        if (!window.confirm(`Permanently delete ${inst.name} and its mods, saves and settings? Back up your worlds first.`)) return;
        await invoke('remove_instance', { instanceId: inst.id }); await syncInstances(); notice = 'Instance removed'; break;
      }
      case 'install-loader': {
        const inst = instances.find(i => i.id === targetId);
        if (!inst) throw new Error('Instance not found');
        await installVersion(inst.versionId);
        await invoke('configure_instance_loader', { instanceId: inst.id, loader: loaderKind, loaderVersion: loaderBuild || null, javaPath });
        await syncInstances(); selectedInstanceId = inst.id;
        modResults = []; notice = `${inst.name}: ${loaderKind} configured`; break;
      }
      case 'mod-search':
        if (!selectedInstanceId) throw new Error('Select an instance first');
        modSearch = searchText;
        modResults = await invoke<ModProject[]>('find_mods', { instanceId: selectedInstanceId, query: modSearch });
        notice = `Found ${modResults.length} compatible projects`; break;
      case 'mod-install':
        if (!selectedInstanceId) throw new Error('Select an instance');
        localMods = await invoke<LocalMod[]>('install_modrinth_mod', { instanceId: selectedInstanceId, projectId: targetId });
        notice = 'Mod and required dependencies installed'; break;
      case 'mod-toggle':
        if (!selectedInstanceId) throw new Error('Select an instance');
        localMods = await invoke<LocalMod[]>('toggle_local_mod', { instanceId: selectedInstanceId, filename: targetId, enabled: element.dataset.enabled === 'true' });
        notice = 'Mod status changed'; break;
      case 'mod-remove':
        if (!selectedInstanceId) throw new Error('Select an instance');
        if (!window.confirm(`Delete ${targetId} from this instance?`)) return;
        localMods = await invoke<LocalMod[]>('remove_local_mod', { instanceId: selectedInstanceId, filename: targetId });
        notice = 'Mod file removed'; break;
      case 'open-instance':
        await invoke('open_instance_folder', { instanceId: targetId, subdirectory: 'game' }); break;
      case 'open-mods':
      case 'open-saves':
        if (!selectedInstanceId) throw new Error('Select an instance');
        await invoke('open_instance_folder', { instanceId: selectedInstanceId, subdirectory: name === 'open-mods' ? 'mods' : 'saves' });
        await syncFiles(); break;
      case 'save-settings': {
        const java = newJava || 'auto';
        const ram = Number(newMemory);
        const appClientId = newClientId;
        if (!Number.isInteger(ram) || ram < 1024 || ram > 32768) throw new Error('RAM must be 1024–32768 MB');
        if (java.length > 1024) throw new Error('Java path is too long');
        javaPath = java; memoryMb = ram; clientId = appClientId;
        showSnapshots = newSnapshots;
        save('java', javaPath); save('ram', memoryMb); save('clientId', clientId); save('snapshots', showSnapshots);
        notice = 'Settings saved'; break;
      }
      default: break;
    }
    status = notice || 'Ready';
    save('accountId', selectedAccountId); save('instanceId', selectedInstanceId); save('version', selectedVersion);
  } catch (e) { fail(e); }
  finally { busy = false; render(); }
}
async function finishLogin(): Promise<void> {
  try {
    accounts = await invoke<Account[]>('complete_login');
    selectedAccountId = activeAccount()?.id || '';
    notice = `Signed in as ${activeAccount()?.name || 'Minecraft account'}`;
    status = 'Account connected'; save('accountId', selectedAccountId);
  } catch (e) { fail(e); }
  finally { loginWaiting = false; loginPrompt = null; render(); }
}
root.addEventListener('click', event => {
  const target = (event.target as HTMLElement).closest<HTMLElement>('[data-action]');
  if (target) { event.preventDefault(); void action(target.dataset.action!, target); }
});
root.addEventListener('submit', event => { event.preventDefault(); });
root.addEventListener('input', event => {
  const target = event.target as HTMLInputElement;
  if (target.id === 'accountSearch') {
    accountSearch = target.value;
    const list = root.querySelector<HTMLElement>('.account-cards');
    if (list) {
      const filtered = accounts.filter(a => a.name.toLowerCase().includes(accountSearch.toLowerCase()));
      list.innerHTML = filtered.map(accountCard).join('') || '<div class="empty-state">No matching accounts</div>';
    }
  }
});
root.addEventListener('change', event => {
  const target = event.target as HTMLInputElement | HTMLSelectElement;
  if (target.id === 'versionPicker') { selectedVersion = target.value; selectedInstanceId = ''; save('version', selectedVersion); render(); }
  if (target.id === 'showSnapshots') { showSnapshots = (target as HTMLInputElement).checked; save('snapshots', showSnapshots); render(); }
  if (target.id === 'playInstance' || target.id === 'activeInstance') {
    selectedInstanceId = target.value; save('instanceId', selectedInstanceId);
    if (target.id === 'playInstance') { render(); }
    else { void syncFiles().then(render).catch(fail); }
  }
  if (target.id === 'accountSearch') { accountSearch = target.value; render(); }
  if (target.id === 'modSearch') { modSearch = target.value; }
});
async function init(): Promise<void> {
  render();
  if (previewMode) {
    versions = [{ id: '1.21.11', type: 'release', url: '' }];
    selectedVersion = '1.21.11';
    accounts = [
      { id: 'demo-ms', name: 'PebblePlayer', uuid: '3e2f1f2a5c8b4d9e9c1a7b8c5e4d2f10', kind: 'microsoft', active: true },
      { id: 'demo-offline', name: 'OfflineTest', uuid: 'a12f94a743b143ad8dc99ebd013e302b', kind: 'offline', active: false },
      { id: 'demo-token', name: 'SessionUser', uuid: '2a9c7f448e1b4c3d9f709e3c5a1b6d28', kind: 'session', active: false },
    ];
    instances = [{ id: 'custom-survival', name: 'Survival', versionId: selectedVersion }];
    selectedAccountId = accounts[0].id; selectedInstanceId = instances[0].id;
    loading = false; status = 'UI preview — no game operations'; render(); return;
  }
  await listen<Progress>('install-progress', ({ payload }) => {
    progress = payload;
    if (page === 'downloads' || page === 'play') render();
  });
  await listen<{ version: string; message: string }>('game-event', ({ payload }) => {
    logs.push(`[${payload.version}] ${payload.message}`);
    if (logs.length > 400) logs = logs.slice(-250);
    if (page === 'console') render();
  });
  const results = await Promise.allSettled([
    invoke<Version[]>('list_versions'),
    invoke<string[]>('list_installed'),
    invoke<Account[]>('list_accounts'),
    invoke<Instance[]>('list_instances'),
  ]);
  if (results[0].status === 'fulfilled') versions = results[0].value;
  else error = `Version manifest unavailable: ${String(results[0].reason)}`;
  if (results[1].status === 'fulfilled') installed = results[1].value;
  if (results[2].status === 'fulfilled') accounts = results[2].value;
  if (results[3].status === 'fulfilled') instances = results[3].value;
  if (!versions.some(v => v.id === selectedVersion) && !installed.includes(selectedVersion)) {
    selectedVersion = versions.find(v => v.type === 'release')?.id || installed[0] || '';
  }
  if (!accounts.some(a => a.id === selectedAccountId)) selectedAccountId = activeAccount()?.id || accounts[0]?.id || '';
  if (!instances.some(i => i.id === selectedInstanceId)) selectedInstanceId = instances[0]?.id || '';
  loading = false;
  status = versions.length ? 'Ready' : 'Offline — installed versions only';
  render();
}
void init().catch(e => { loading = false; fail(e); render(); });
