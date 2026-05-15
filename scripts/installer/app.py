from pathlib import Path
import sys
import tkinter as tk
from tkinter import filedialog, messagebox

from config import APP_NAME
from engine import update_controller_pack, get_local_version, get_github_version

try:
    from build_info import BUILD_COMMIT, BUILD_BRANCH
except Exception:
    BUILD_COMMIT = "dev"
    BUILD_BRANCH = "local"


GREEN = "#2d7d46"
AMBER = "#d08b00"
GRAY = "#777777"


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


class InstallerApp:
    def __init__(self, root):
        self.root = root
        self.root.title(APP_NAME)

        icon_path = resource_path("ProfileConfigurator.ico")
        if icon_path.exists():
            self.root.iconbitmap(str(icon_path))

        self.root.geometry("700x500")
        self.root.resizable(False, False)

        self.install_dir = tk.StringVar()
        self.version_text = tk.StringVar(
            value="Installed version: Checking... | GitHub version: Checking..."
        )
        self.package_text = tk.StringVar(value="No install/update packages selected")

        self.github_version = "Unknown"
        self.local_version = "Not selected"
        self.update_available = False
        self.gng_packages: list[Path] = []

        tk.Label(
            root,
            text="Controller Pack Installer",
            font=("Segoe UI", 16, "bold"),
        ).pack(pady=14)

        tk.Label(
            root,
            text=(
                "Select your Controller Pack Directory and either install "
                "or update packages for AIRAC updates."
            ),
            wraplength=640,
        ).pack(pady=4)

        tk.Button(
            root,
            text="Controller Pack Directory",
            command=self.select_install_folder,
            height=2,
        ).pack(fill="x", padx=40, pady=8)

        tk.Label(
            root,
            textvariable=self.install_dir,
            wraplength=640,
            fg="gray",
        ).pack()

        tk.Button(
            root,
            text="Install/Update Packages",
            command=self.select_gng_packages,
            height=2,
        ).pack(fill="x", padx=40, pady=8)

        tk.Label(
            root,
            textvariable=self.package_text,
            fg="gray",
        ).pack()

        self.action_button = tk.Button(
            root,
            text="Update Controller Pack from GitHub",
            command=self.run_update,
            height=2,
            bg=GREEN,
            fg="white",
        )
        self.action_button.pack(fill="x", padx=40, pady=18)

        tk.Label(
            root,
            textvariable=self.version_text,
            fg="gray",
            wraplength=640,
        ).pack()

        tk.Label(
            root,
            text=f"Installer build: {BUILD_COMMIT} ({BUILD_BRANCH})",
            fg="gray",
        ).pack(pady=6)

        self.auto_detect_start_directory()
        self.refresh_versions()
        self.refresh_action_button()

    def auto_detect_start_directory(self):
        start_dir = get_start_directory()

        if looks_like_controller_pack(start_dir):
            self.install_dir.set(str(start_dir))

    def selected_package_mode(self) -> str:
        if not self.gng_packages:
            return "github"

        names = " ".join(package.name.lower() for package in self.gng_packages)

        if "install" in names:
            return "install"

        if "update" in names:
            return "update_gng"

        return "update_gng"

    def refresh_action_button(self):
        mode = self.selected_package_mode()
        count = len(self.gng_packages)

        if mode == "install":
            self.action_button.config(text="Install Controller Pack")
            self.package_text.set(f"Install package detected — {count} package(s) selected")

        elif mode == "update_gng":
            self.action_button.config(text="Update Controller Pack from GitHub + GNG")
            self.package_text.set(f"Update package detected — {count} package(s) selected")

        else:
            self.action_button.config(text="Update Controller Pack from GitHub")
            self.package_text.set("No install/update packages selected")

        if self.github_version in ["Unknown", "Unable to check"]:
            self.action_button.config(bg=GRAY)

        elif self.update_available:
            self.action_button.config(bg=AMBER)

        else:
            self.action_button.config(bg=GREEN)

    def refresh_versions(self):
        install_root = Path(self.install_dir.get()) if self.install_dir.get() else None

        try:
            self.github_version = get_github_version()
        except Exception:
            self.github_version = "Unable to check"

        if install_root:
            try:
                self.local_version = get_local_version(install_root)
            except Exception:
                self.local_version = "Unknown"
        else:
            self.local_version = "Not selected"

        self.update_available = (
            self.github_version not in ["Unknown", "Unable to check"]
            and self.local_version not in ["Not selected", "Not installed", "Unknown"]
            and self.local_version != self.github_version
        )

        if self.local_version == "Not installed":
            status = "Not installed"

        elif self.github_version == "Unable to check":
            status = "Could not check GitHub"

        elif self.update_available:
            status = "Update available"

        elif self.local_version == self.github_version:
            status = "Up to date"

        else:
            status = "Ready"

        self.version_text.set(
            f"Installed version: {self.local_version} | "
            f"GitHub version: {self.github_version} | "
            f"Status: {status}"
        )

    def select_install_folder(self):
        folder = filedialog.askdirectory(title="Select controller pack directory")

        if folder:
            self.install_dir.set(folder)
            self.refresh_versions()
            self.refresh_action_button()

    def select_gng_packages(self):
        files = filedialog.askopenfilenames(
            title="Select install/update packages",
            filetypes=[
                ("Archive files", "*.zip *.7z"),
                ("ZIP files", "*.zip"),
                ("7Z files", "*.7z"),
            ],
        )

        self.gng_packages = [Path(file) for file in files]
        self.refresh_action_button()


    def run_update(self):
        if not self.install_dir.get():
            messagebox.showerror(
                "Missing directory",
                "Please select the controller pack directory first.",
            )
            return

        install_root = Path(self.install_dir.get())
        mode = self.selected_package_mode()

        try:
            self.root.config(cursor="wait")
            self.root.update_idletasks()

            update_controller_pack(
                install_root=install_root,
                gng_packages=self.gng_packages,
            )

            self.refresh_versions()
            self.refresh_action_button()

            if mode == "install":
                message = "Controller pack installation completed successfully."
            elif mode == "update_gng":
                message = "Controller pack GitHub + GNG update completed successfully."
            else:
                message = "Controller pack GitHub update completed successfully."

            messagebox.showinfo("Complete", message)

        except Exception as error:
            messagebox.showerror("Failed", str(error))

        finally:
            self.root.config(cursor="")
            self.root.update_idletasks()


if __name__ == "__main__":
    root = tk.Tk()
    app = InstallerApp(root)
    root.mainloop()