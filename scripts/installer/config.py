FIRS = ["LFBB", "LFEE", "LFFF", "LFMM", "LFRR"]

REPO_LAYOUT_MAP = {
    "LFBB": "LFBB",
    "LFEE": "LFEE",
    "LFFF": "LFFF",
    "LFMM": "LFMM",
    "LFRR": "LFRR",
    "LFXX": "LFXX",
}

GNG_ONLY_FILES = [
    # Protect ALL sector files/folders from GitHub sync.
    "LFXX/Sectors",
    "LFXX/Sectors/*",
    "LFXX/Sectors/*/*",

    # Alias comes from GNG.
    "LFXX/Alias",
    "LFXX/Alias/*",
    
    # LFXX generated/navdata files from GNG.
    "LFXX/ICAO",
    "LFXX/ICAO/*",
    "LFXX/NavData",
    "LFXX/NavData/*",

    # CoFrance plugin generated files.
    "LFXX/Plugins/CoFrance",
    "LFXX/Plugins/CoFrance/*",

    # Settings backups.
    "LFXX/Settings/settings_backup",
    "LFXX/Settings/settings_backup/*",
    "LFXX/Settings/settings_backup/*/*",

    # FIR data from GNG only.
    "LFBB/ICAO",
    "LFBB/ICAO/*",
    "LFBB/NavData",
    "LFBB/NavData/*",
    "LFBB/Settings/LoginProfiles.txt",
    "LFBB/Settings/VoiceChannels.txt",

    "LFEE/ICAO",
    "LFEE/ICAO/*",
    "LFEE/NavData",
    "LFEE/NavData/*",
    "LFEE/Settings/LoginProfiles.txt",
    "LFEE/Settings/VoiceChannels.txt",

    "LFFF/ICAO",
    "LFFF/ICAO/*",
    "LFFF/NavData",
    "LFFF/NavData/*",
    "LFFF/Settings/LoginProfiles.txt",
    "LFFF/Settings/VoiceChannels.txt",

    "LFMM/ICAO",
    "LFMM/ICAO/*",
    "LFMM/NavData",
    "LFMM/NavData/*",
    "LFMM/Settings/LoginProfiles.txt",
    "LFMM/Settings/VoiceChannels.txt",

    "LFRR/ICAO",
    "LFRR/ICAO/*",
    "LFRR/NavData",
    "LFRR/NavData/*",
    "LFRR/Settings/LoginProfiles.txt",
    "LFRR/Settings/VoiceChannels.txt",
]

GITHUB_OWNER = "vaccfr"
GITHUB_REPO = "Sector-Files"
GITHUB_BRANCH = "main"

APP_NAME = "Controller Pack Installer"
VERSION_FILE = ".github/installer-version.txt"