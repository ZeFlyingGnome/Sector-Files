import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  api,
  onEvent,
  type CheckUpdatesReport,
  type FirCode,
  type GngStatus,
  type Profile,
  type SyncSummary,
} from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { UpdateBanner } from "@/components/UpdateBanner";
import { CircleAlert, Download, LogIn, RefreshCcw, Save } from "lucide-react";

const RATINGS = ["OBS", "S1", "S2", "S3", "C1", "C2", "C3", "I1", "I2", "I3", "SUP", "ADM"] as const;
const ALL_FIRS: FirCode[] = ["LFBB", "LFEE", "LFFF", "LFMM", "LFRR"];

type Tab = "sync" | "profile" | "settings";

export default function App() {
  const [profile, setProfile] = useState<Profile | null>(null);
  const [gngStatus, setGngStatus] = useState<GngStatus>({ signed_in: false, username: null });
  const [updateStatus, setUpdateStatus] = useState<CheckUpdatesReport | null>(null);
  const [tab, setTab] = useState<Tab>("sync");
  const [busy, setBusy] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  useEffect(() => {
    api.getProfile().then(async (p) => {
      if (!p.controller_pack_dir) {
        const detected = await api.detectPackDir();
        if (detected) {
          p = await api.updateProfile({ controller_pack_dir: detected });
        }
      }
      setProfile(p);
    });
    api.gngStatus().then(setGngStatus).catch(() => {});
    api.checkUpdates().then(setUpdateStatus).catch(() => {});

    let unlistenUpdates: (() => void) | undefined;
    let unlistenSync: (() => void) | undefined;
    onEvent<CheckUpdatesReport>("updates:report", setUpdateStatus).then((u) => (unlistenUpdates = u));
    onEvent<{ step: string }>("sync:progress", (p) => setToast(p.step)).then((u) => (unlistenSync = u));
    return () => {
      unlistenUpdates?.();
      unlistenSync?.();
    };
  }, []);

  const pickPackDir = useCallback(async () => {
    const dir = await open({ directory: true, multiple: false, title: "Select controller pack directory" });
    if (typeof dir === "string") {
      const updated = await api.updateProfile({ controller_pack_dir: dir });
      setProfile(updated);
    }
  }, []);

  const runSync = useCallback(async () => {
    if (!profile?.controller_pack_dir) {
      setToast("Set the controller pack directory first.");
      return;
    }
    setBusy(true);
    try {
      const summary: SyncSummary = await api.runSync(profile.preferences.selected_firs);
      setToast(
        `Sync complete — ${summary.files_written} files written` +
          (summary.warnings.length ? `, ${summary.warnings.length} warning(s)` : ""),
      );
      const refreshed = await api.getProfile();
      setProfile(refreshed);
      const refreshedUpdates = await api.checkUpdates();
      setUpdateStatus(refreshedUpdates);
    } catch (e) {
      setToast(`Sync failed: ${e}`);
    } finally {
      setBusy(false);
    }
  }, [profile]);

  const signIn = useCallback(async () => {
    await api.openGngLogin();
    setTimeout(async () => setGngStatus(await api.gngStatus()), 1000);
  }, []);

  if (!profile) {
    return <div className="p-8 text-neutral-400">Loading…</div>;
  }

  return (
    <div className="flex flex-col h-screen">
      <UpdateBanner />
      <Header
        tab={tab}
        setTab={setTab}
        gng={gngStatus}
        updates={updateStatus}
      />
      <main className="flex-1 overflow-y-auto px-8 py-6 space-y-6">
        {tab === "sync" && (
          <SyncPanel
            profile={profile}
            onPickDir={pickPackDir}
            onSignIn={signIn}
            onRun={runSync}
            busy={busy}
            gng={gngStatus}
          />
        )}
        {tab === "profile" && <ProfilePanel profile={profile} setProfile={setProfile} setToast={setToast} />}
        {tab === "settings" && <SettingsPanel profile={profile} setProfile={setProfile} />}
      </main>
      {toast && <Toast text={toast} onDismiss={() => setToast(null)} />}
    </div>
  );
}

function Header({
  tab,
  setTab,
  gng,
  updates,
}: {
  tab: Tab;
  setTab: (t: Tab) => void;
  gng: GngStatus;
  updates: CheckUpdatesReport | null;
}) {
  return (
    <header className="border-b border-neutral-800 px-6 py-3 flex items-center gap-6">
      <h1 className="font-semibold text-base">Controller Pack Installer</h1>
      <nav className="flex gap-2 flex-1">
        {(["sync", "profile", "settings"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`px-3 py-1.5 rounded-md text-sm capitalize ${
              tab === t ? "bg-neutral-800 text-white" : "text-neutral-400 hover:text-white"
            }`}
          >
            {t}
          </button>
        ))}
      </nav>
      <UpdateBadge updates={updates} />
      <span className="text-xs text-neutral-400">
        GNG: {gng.signed_in ? `✓ ${gng.username ?? "signed in"}` : "not signed in"}
      </span>
    </header>
  );
}

function UpdateBadge({ updates }: { updates: CheckUpdatesReport | null }) {
  if (!updates) return <span className="text-xs text-neutral-500">…</span>;
  const pill = (label: string, status: CheckUpdatesReport["github"]) => {
    const colour =
      status.kind === "up_to_date"
        ? "bg-emerald-700"
        : status.kind === "update_available"
        ? "bg-amber-600"
        : "bg-neutral-700";
    return (
      <span key={label} className={`px-2 py-0.5 rounded text-[10px] uppercase tracking-wide text-white ${colour}`}>
        {label}: {status.kind === "update_available" ? status.value : status.kind === "up_to_date" ? "ok" : "?"}
      </span>
    );
  };
  return (
    <div className="flex gap-1">
      {pill("github", updates.github)}
      {pill("airac", updates.airac)}
    </div>
  );
}

function SyncPanel({
  profile,
  onPickDir,
  onSignIn,
  onRun,
  busy,
  gng,
}: {
  profile: Profile;
  onPickDir: () => void;
  onSignIn: () => void;
  onRun: () => void;
  busy: boolean;
  gng: GngStatus;
}) {
  return (
    <div className="space-y-4 max-w-xl">
      <Row label="Controller pack directory">
        <div className="flex gap-2">
          <input
            value={profile.controller_pack_dir ?? ""}
            readOnly
            placeholder="Not set"
            className="flex-1 bg-neutral-900 border border-neutral-800 rounded px-2 py-1 text-sm"
          />
          <Button variant="outline" size="sm" onClick={onPickDir}>
            Choose…
          </Button>
        </div>
      </Row>
      <Row label="AeroNav (GNG) sign-in">
        <div className="flex items-center gap-3">
          <span className="text-sm">{gng.signed_in ? `Signed in as ${gng.username ?? "?"}` : "Not signed in"}</span>
          <Button variant="secondary" size="sm" onClick={onSignIn}>
            <LogIn className="w-4 h-4" /> {gng.signed_in ? "Sign in again" : "Sign in to AeroNav"}
          </Button>
        </div>
      </Row>
      <Button onClick={onRun} disabled={busy || !profile.controller_pack_dir} size="lg">
        {busy ? <RefreshCcw className="w-4 h-4 animate-spin" /> : <Download className="w-4 h-4" />}
        {busy ? "Syncing…" : "Sync now"}
      </Button>
      <p className="text-xs text-neutral-500 max-w-md flex items-start gap-2">
        <CircleAlert className="w-3.5 h-3.5 mt-0.5 shrink-0" />
        Sync downloads the latest configuration from GitHub and the latest AIRAC files from GNG, then applies your profile if configured.
      </p>
    </div>
  );
}

function ProfilePanel({
  profile,
  setProfile,
  setToast,
}: {
  profile: Profile;
  setProfile: (p: Profile) => void;
  setToast: (s: string) => void;
}) {
  const [draft, setDraft] = useState(profile.vatsim);
  useEffect(() => setDraft(profile.vatsim), [profile.vatsim]);
  const dirty = useMemo(() => JSON.stringify(draft) !== JSON.stringify(profile.vatsim), [draft, profile.vatsim]);

  const save = async () => {
    const updated = await api.updateProfile({ vatsim: draft });
    setProfile(updated);
    setToast("Profile saved");
  };

  const applyNow = async () => {
    if (!profile.controller_pack_dir) return setToast("Set controller pack directory first");
    const count = await api.applyProfileToPack(profile.controller_pack_dir);
    setToast(`Patched ${count} file(s)`);
  };

  return (
    <div className="space-y-4 max-w-xl">
      <div className="rounded border border-amber-700/40 bg-amber-700/10 px-3 py-2 text-xs text-amber-200">
        Credentials are stored unencrypted on disk in this app's data directory. The VATSIM password is also written
        in plain text into every <code>.prf</code> file by EuroScope.
      </div>
      <Row label="Real name">
        <Field value={draft.real_name} onChange={(v) => setDraft({ ...draft, real_name: v })} />
      </Row>
      <Row label="VATSIM CID">
        <Field value={draft.cid} onChange={(v) => setDraft({ ...draft, cid: v })} placeholder="1234567" />
      </Row>
      <Row label="Password">
        <Field value={draft.password} onChange={(v) => setDraft({ ...draft, password: v })} type="password" />
      </Row>
      <Row label="Rating">
        <select
          value={draft.rating}
          onChange={(e) => setDraft({ ...draft, rating: e.target.value })}
          className="bg-neutral-900 border border-neutral-800 rounded px-2 py-1 text-sm"
        >
          {RATINGS.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>
      </Row>
      <Row label="EuroScopeRPC">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={draft.enable_rpc}
            onChange={(e) => setDraft({ ...draft, enable_rpc: e.target.checked })}
          />
          Enable Discord Rich Presence plugin
        </label>
      </Row>
      <div className="flex gap-2">
        <Button onClick={save} disabled={!dirty}>
          <Save className="w-4 h-4" /> Save
        </Button>
        <Button variant="outline" onClick={applyNow} disabled={!profile.controller_pack_dir}>
          Apply now to installed pack
        </Button>
      </div>
    </div>
  );
}

function SettingsPanel({ profile, setProfile }: { profile: Profile; setProfile: (p: Profile) => void }) {
  const togglePref = async (key: "auto_check_updates" | "apply_creds_after_sync") => {
    const prefs = { ...profile.preferences, [key]: !profile.preferences[key] };
    const updated = await api.updateProfile({ preferences: prefs });
    setProfile(updated);
  };
  const toggleFir = async (fir: FirCode) => {
    const set = new Set(profile.preferences.selected_firs);
    set.has(fir) ? set.delete(fir) : set.add(fir);
    const prefs = { ...profile.preferences, selected_firs: Array.from(set) };
    const updated = await api.updateProfile({ preferences: prefs });
    setProfile(updated);
  };
  return (
    <div className="space-y-4 max-w-xl">
      <Row label="FIRs to install/update">
        <div className="flex flex-wrap gap-2">
          {ALL_FIRS.map((fir) => (
            <label key={fir} className="flex items-center gap-1 text-sm">
              <input
                type="checkbox"
                checked={profile.preferences.selected_firs.includes(fir)}
                onChange={() => toggleFir(fir)}
              />
              {fir}
            </label>
          ))}
        </div>
      </Row>
      <Row label="Apply credentials to .prf files after sync">
        <input
          type="checkbox"
          checked={profile.preferences.apply_creds_after_sync}
          onChange={() => togglePref("apply_creds_after_sync")}
        />
      </Row>
      <Row label="Auto-check for updates every 30 minutes">
        <input
          type="checkbox"
          checked={profile.preferences.auto_check_updates}
          onChange={() => togglePref("auto_check_updates")}
        />
      </Row>
      <Row label="Installed versions">
        <div className="text-xs text-neutral-400">
          GitHub: {profile.versions.installed_github_sha ?? "—"} | AIRAC:{" "}
          {profile.versions.installed_airac_cycle ?? "—"}
        </div>
      </Row>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[180px_1fr] items-center gap-3">
      <label className="text-sm text-neutral-300">{label}</label>
      <div>{children}</div>
    </div>
  );
}

function Field({
  value,
  onChange,
  type = "text",
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
}) {
  return (
    <input
      type={type}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className="w-full bg-neutral-900 border border-neutral-800 rounded px-2 py-1 text-sm"
    />
  );
}

function Toast({ text, onDismiss }: { text: string; onDismiss: () => void }) {
  useEffect(() => {
    const t = setTimeout(onDismiss, 4000);
    return () => clearTimeout(t);
  }, [onDismiss]);
  return (
    <div className="fixed bottom-4 right-4 bg-neutral-800 border border-neutral-700 rounded px-4 py-2 text-sm shadow-xl">
      {text}
    </div>
  );
}
