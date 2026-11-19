# Signing the Android app

The release workflow (`.github/workflows/release.yml`) signs the Android `.apk` and `.aab` with
the app's **release key**. The key never goes into the repository: it is stored as secrets of the
GitHub environment `release` (REQUIREMENTS.md R-704).

Without the secrets the workflow still builds, but the `.apk` uses a debug key that is new on
every build (so it cannot update an app installed from an earlier release), and the `.aab` is
unsigned (Google Play does not accept it).

## 1. Install Java (once)

`keytool`, the tool that makes the key, comes with Java. On Windows, in PowerShell:

```powershell
winget install --id EclipseAdoptium.Temurin.21.JDK
```

Close PowerShell and open a new one (so it finds Java), then check:

```powershell
keytool -help
```

If PowerShell does not find `keytool`, call it with its full path, for example
`& "C:\Program Files\Eclipse Adoptium\jdk-21.0.x-hotspot\bin\keytool.exe"` (the version folder
differs).

## 2. Create the key (once)

Go to a folder outside the repository (for example your Documents) and run:

```powershell
cd $HOME\Documents
keytool -genkeypair -v -keystore theo-manager-release.jks -alias theo-manager `
  -keyalg RSA -keysize 4096 -validity 10000
```

It asks for a password, then your name and organisation (any values are fine; they are shown
only in the certificate). Current Java versions ask for **one** password: it protects both the
keystore and the key. Keep the file **outside** the repository folder.

> **Back it up.** Store `theo-manager-release.jks`, the alias and both passwords in your password
> manager. If you lose the key, phones that have the app cannot install updates any more: people
> would have to uninstall it first (and Google Play only accepts updates signed with the same key,
> unless you use Play App Signing).

## 3. Add the secrets to GitHub

1. Make the key one line of text (PowerShell, in the folder of the key; it goes to the clipboard):

   ```powershell
   [Convert]::ToBase64String([IO.File]::ReadAllBytes("$PWD	heo-manager-release.jks")) | Set-Clipboard
   ```

2. In the repository on GitHub: **Settings → Environments → New environment**, name it `release`.
   Optional but recommended: tick **Required reviewers** and add yourself, so a release waits for
   your approval before it can use the key.
3. In the `release` environment, under **Environment secrets**, add:

   | Secret | Value |
   |---|---|
   | `ANDROID_KEYSTORE_BASE64` | The text you copied (paste it) |
   | `ANDROID_KEYSTORE_PASSWORD` | The keystore password |
   | `ANDROID_KEY_ALIAS` | `theo-manager` (the `-alias` you chose) |
   | `ANDROID_KEY_PASSWORD` | The same password again (or leave this secret out: the keystore password is used) |

## 4. Release

Publish a release as usual. The Android job then:

1. sets the Android version code from the tag (`v1.2.3` → `10203`; Android installs an update only
   when the number is higher),
2. builds the release `.apk` and `.aab`,
3. signs the `.apk` (`zipalign` and `apksigner`) and the `.aab` (`jarsigner`) and checks both
   signatures.

If you added a required reviewer, approve the waiting jobs in the run (**Review deployments**).

## If a phone does not install the `.apk`

- **An earlier build is installed** with another key (every unsigned build had a new debug key):
  uninstall the app once, then install the signed `.apk`. Later signed releases update it.
- **"Install unknown apps"** must be allowed for the browser or file manager you open the `.apk`
  from (Android asks the first time).
- **Play Protect** may warn about an app it does not know: choose *Install anyway*.
