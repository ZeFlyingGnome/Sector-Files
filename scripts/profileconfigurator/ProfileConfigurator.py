from pathlib import Path
import re
import sys
import tkinter as tk
from tkinter import filedialog, messagebox, ttk


APP_NAME = "Profile Configurator"
GREEN = "#2d7d46"
AMBER = "#d08b00"
GRAY = "#777777"

RPC_PATH = r"\..\LFXX\Plugins\EuroscopeRPC\EuroscopeRPC.dll"

RATINGS = [
    "OBS", "S1", "S2", "S3", "C1", "C2",
    "C3", "I1", "I2", "I3", "SUP", "ADM",
]

try:
    from build_info import BUILD_COMMIT, BUILD_BRANCH
except Exception:
    BUILD_COMMIT = "dev"
    BUILD_BRANCH = "local"


def resource_path(relative_path: str) -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys._MEIPASS) / relative_path
    return Path(__file__).resolve().parent / relative_path


def get_start_directory() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).resolve().parent
    return Path.cwd()


def looks_like_controller_pack(path: Path) -> bool:
    return (
        (path / "LFXX").exists()
        and any((path / fir).exists() for fir in ["LFBB", "LFEE", "LFFF", "LFMM", "LFRR"])
    )


def find_prf_files(root: Path) -> list[Path]:
    return sorted(path for path in root.rglob("*.prf") if path.is_file())


def find_login_profiles_files(root: Path) -> list[Path]:
    return sorted(path for path in root.rglob("LoginProfiles.txt") if path.is_file())


def rating_to_euroscope_value(rating: str) -> str:
    try:
        return str(RATINGS.index(rating.strip().upper()))
    except ValueError:
        return "1"


def is_euroscope_rpc_line(line: str) -> bool:
    stripped = line.strip()

    if not stripped.startswith("Plugins\tPlugin"):
        return False

    normalized = stripped.replace("/", "\\").lower()
    return "lfxx\\plugins\\euroscoperpc\\euroscoperpc.dll" in normalized


def ensure_rpc_plugin(lines: list[str]) -> list[str]:
    for line in lines:
        if is_euroscope_rpc_line(line):
            return lines

    plugin_numbers = []

    for line in lines:
        match = re.match(r"Plugins\tPlugin(\d+)\t", line)
        if match:
            plugin_numbers.append(int(match.group(1)))

    next_plugin = max(plugin_numbers, default=0) + 1

    if lines and not lines[-1].endswith(("\n", "\r")):
        lines[-1] += "\n"

    lines.append(f"Plugins\tPlugin{next_plugin}\t{RPC_PATH}\n")
    return lines


def patch_prf_file(path: Path, details: dict) -> bool:
    try:
        original = path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return False

    lines = original.splitlines(keepends=True)
    output = []

    found_realname = False
    found_certificate = False
    found_rating = False
    found_password = False

    for line in lines:
        stripped = line.rstrip("\r\n")

        if is_euroscope_rpc_line(stripped):
            if not details["enable_rpc"]:
                continue

        if stripped.startswith("LastSession\trealname\t"):
            output.append(f"LastSession\trealname\t{details['name']}\n")
            found_realname = True
            continue

        if stripped.startswith("LastSession\tcertificate\t"):
            output.append(f"LastSession\tcertificate\t{details['cid']}\n")
            found_certificate = True
            continue

        if stripped.startswith("LastSession\trating\t"):
            output.append(f"LastSession\trating\t{details['rating']}\n")
            found_rating = True
            continue

        if stripped.startswith("LastSession\tpassword\t"):
            output.append(f"LastSession\tpassword\t{details['password']}\n")
            found_password = True
            continue

        output.append(line)

    if output and not output[-1].endswith(("\n", "\r")):
        output[-1] += "\n"

    if not found_realname:
        output.append(f"LastSession\trealname\t{details['name']}\n")

    if not found_certificate:
        output.append(f"LastSession\tcertificate\t{details['cid']}\n")

    if not found_rating:
        output.append(f"LastSession\trating\t{details['rating']}\n")

    if not found_password:
        output.append(f"LastSession\tpassword\t{details['password']}\n")

    if details["enable_rpc"]:
        output = ensure_rpc_plugin(output)

    updated = "".join(output)

    if updated == original:
        return False

    path.write_text(updated, encoding="utf-8", newline="")
    return True


def patch_login_profiles_file(path: Path, cid: str) -> bool:
    try:
        original = path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return False

    updated = original.replace("VOTRE_CID_ICI", cid)

    if updated == original:
        return False

    path.write_text(updated, encoding="utf-8", newline="")
    return True


class ProfileConfiguratorApp:
    def __init__(self, root):
        self.root = root
        self.root.title(APP_NAME)

        icon_path = resource_path("ProfileConfigurator.ico")
        if icon_path.exists():
            self.root.iconbitmap(str(icon_path))

        self.root.geometry("720x620")
        self.root.resizable(False, False)

        self.controller_pack_dir = tk.StringVar()
        self.status_text = tk.StringVar(value="No controller pack directory selected")
        self.prf_count_text = tk.StringVar(value="Profiles detected: 0")

        self.name_var = tk.StringVar()
        self.cid_var = tk.StringVar()
        self.password_var = tk.StringVar()
        self.rating_var = tk.StringVar(value="S1")
        self.rpc_var = tk.BooleanVar(value=True)

        tk.Label(
            root,
            text="Profile Configurator",
            font=("Segoe UI", 16, "bold"),
        ).pack(pady=14)

        tk.Label(
            root,
            text=(
                "Update EuroScope login details, rating, EuroScopeRPC settings, "
                "and LoginProfiles feedback links for your controller pack."
            ),
            wraplength=660,
        ).pack(pady=4)

        tk.Button(
            root,
            text="Controller Pack Directory",
            command=self.select_controller_pack_directory,
            height=2,
        ).pack(fill="x", padx=40, pady=8)

        tk.Label(
            root,
            textvariable=self.controller_pack_dir,
            wraplength=660,
            fg="gray",
        ).pack()

        form = ttk.Frame(root, padding=20)
        form.pack(fill="x")

        ttk.Label(form, text="Controller Name").grid(row=0, column=0, sticky="w", pady=6)
        ttk.Entry(form, textvariable=self.name_var, width=40).grid(row=0, column=1, sticky="ew")

        ttk.Label(form, text="CID").grid(row=1, column=0, sticky="w", pady=6)
        ttk.Entry(form, textvariable=self.cid_var, width=40).grid(row=1, column=1, sticky="ew")

        ttk.Label(form, text="Password").grid(row=2, column=0, sticky="w", pady=6)
        ttk.Entry(form, textvariable=self.password_var, show="*", width=40).grid(row=2, column=1, sticky="ew")

        ttk.Label(form, text="Controller Rating").grid(row=3, column=0, sticky="w", pady=6)
        ttk.Combobox(
            form,
            values=RATINGS,
            textvariable=self.rating_var,
            state="readonly",
            width=37,
        ).grid(row=3, column=1, sticky="ew")

        ttk.Checkbutton(
            form,
            text="Enable EuroScopeRPC / Discord Rich Presence",
            variable=self.rpc_var,
        ).grid(row=4, column=0, columnspan=2, sticky="w", pady=10)

        form.columnconfigure(1, weight=1)

        tk.Label(
            root,
            textvariable=self.prf_count_text,
            fg="gray",
        ).pack(pady=8)

        self.action_button = tk.Button(
            root,
            text="Update Profile Configuration",
            command=self.run_profile_update,
            height=2,
            bg=GREEN,
            fg="white",
        )
        self.action_button.pack(fill="x", padx=40, pady=18)

        tk.Label(
            root,
            textvariable=self.status_text,
            fg="gray",
            wraplength=660,
        ).pack()

        tk.Label(
            root,
            text=f"ProfileConfigurator build: {BUILD_COMMIT} ({BUILD_BRANCH})",
            fg="gray",
        ).pack(pady=6)

        self.auto_detect_start_directory()
        self.refresh_status()

    def auto_detect_start_directory(self):
        start_dir = get_start_directory()

        if looks_like_controller_pack(start_dir):
            self.controller_pack_dir.set(str(start_dir))

    def refresh_status(self):
        if not self.controller_pack_dir.get():
            self.status_text.set("No controller pack directory selected")
            self.prf_count_text.set("Profiles detected: 0")
            self.action_button.config(bg=GRAY)
            return

        path = Path(self.controller_pack_dir.get())

        if not path.exists():
            self.status_text.set("Selected directory does not exist")
            self.prf_count_text.set("Profiles detected: 0")
            self.action_button.config(bg=GRAY)
            return

        if not looks_like_controller_pack(path):
            self.status_text.set("Selected folder does not look like a CoFrance Controller Pack")
            self.action_button.config(bg=AMBER)
        else:
            self.status_text.set("Ready")
            self.action_button.config(bg=GREEN)

        prf_count = len(find_prf_files(path))
        login_count = len(find_login_profiles_files(path))
        self.prf_count_text.set(
            f"Profiles detected: {prf_count} | LoginProfiles detected: {login_count}"
        )

    def select_controller_pack_directory(self):
        folder = filedialog.askdirectory(title="Select controller pack directory")

        if folder:
            self.controller_pack_dir.set(folder)
            self.refresh_status()

    def validate_inputs(self):
        if not self.name_var.get().strip():
            raise ValueError("Name cannot be blank")

        cid = self.cid_var.get().strip()

        if not cid.isdigit() or not (6 <= len(cid) <= 8):
            raise ValueError("CID must be 6 to 8 digits")

    def run_profile_update(self):
        if not self.controller_pack_dir.get():
            messagebox.showerror(
                "Missing directory",
                "Please select the controller pack directory first.",
            )
            return

        try:
            self.validate_inputs()

            self.root.config(cursor="wait")
            self.root.update_idletasks()

            details = {
                "name": self.name_var.get().strip(),
                "cid": self.cid_var.get().strip(),
                "password": self.password_var.get(),
                "rating": rating_to_euroscope_value(self.rating_var.get()),
                "enable_rpc": self.rpc_var.get(),
            }

            controller_pack_dir = Path(self.controller_pack_dir.get())

            prf_files = find_prf_files(controller_pack_dir)
            login_profiles_files = find_login_profiles_files(controller_pack_dir)

            prf_changed = 0
            login_profiles_changed = 0

            for prf in prf_files:
                if patch_prf_file(prf, details):
                    prf_changed += 1

            for login_file in login_profiles_files:
                if patch_login_profiles_file(login_file, details["cid"]):
                    login_profiles_changed += 1

            self.refresh_status()

            messagebox.showinfo(
                "Complete",
                f"Finished updating profile files.\n\n"
                f"PRFs found: {len(prf_files)}\n"
                f"PRFs modified: {prf_changed}\n"
                f"LoginProfiles found: {len(login_profiles_files)}\n"
                f"LoginProfiles modified: {login_profiles_changed}",
            )

        except Exception as error:
            messagebox.showerror("Failed", str(error))

        finally:
            self.root.config(cursor="")
            self.root.update_idletasks()


if __name__ == "__main__":
    root = tk.Tk()
    app = ProfileConfiguratorApp(root)
    root.mainloop()