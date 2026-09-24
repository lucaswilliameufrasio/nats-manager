# macOS Gatekeeper and release signing

## Open the current unsigned release

The NATS Manager `.app.zip` and `.dmg` are currently unsigned and not notarized. Gatekeeper may report that NATS Manager is damaged or cannot be verified. Only use this workaround for an asset downloaded from the official NATS Manager GitHub Release that you trust.

1. Open the downloaded DMG and drag **NATS Manager.app** to `/Applications`.
2. In Finder, Control-click **NATS Manager.app**, choose **Open**, then confirm **Open** in the prompt.
3. If macOS still reports the app as damaged, remove the quarantine attribute from that installed app and open it again:

   ```sh
   xattr -dr com.apple.quarantine "/Applications/NATS Manager.app"
   ```

This removes Gatekeeper's download quarantine for that app; it does not add a code signature or notarization ticket. The current `cargo-bundle` workflow creates the app bundle and DMG without signing or notarizing them.

## Enable signed and notarized releases

For a normal double-click installation without a Gatekeeper bypass, releases must be signed with an Apple Developer ID Application certificate and notarized by Apple.

### Required Apple account material

- An active Apple Developer Program membership and a **Developer ID Application** certificate, exported as a password-protected `.p12` including its private key.
- An App Store Connect API key authorized for notarization (Key ID, Issuer ID, and private `.p8` key).
- The Apple Developer Team ID associated with the certificate and API key.

### GitHub Actions secrets

Add these repository Actions secrets. Store encoded file contents only as GitHub secrets; never commit certificate or private-key files.

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE_P12_BASE64` | Base64-encoded Developer ID `.p12` file |
| `APPLE_CERTIFICATE_PASSWORD` | Password used to export the `.p12` |
| `APPLE_TEAM_ID` | Apple Developer Team ID |
| `APPLE_KEYCHAIN_PASSWORD` | Random password for the temporary CI keychain |
| `APPLE_API_KEY_ID` | App Store Connect API Key ID |
| `APPLE_API_ISSUER` | App Store Connect Issuer ID |
| `APPLE_API_PRIVATE_KEY_BASE64` | Base64-encoded `.p8` API key |

### Release workflow changes

On the macOS runner, the release job should:

1. Create and unlock a temporary keychain, import the `.p12`, and select the **Developer ID Application** identity for `APPLE_TEAM_ID`.
2. Sign `NATS Manager.app` with hardened runtime and a secure timestamp, then verify it with `codesign --verify --deep --strict`.
3. Package the signed app, submit the app archive or DMG with `xcrun notarytool submit --wait` using the App Store Connect API key, and staple the accepted ticket with `xcrun stapler staple`.
4. Run `xcrun stapler validate` and `spctl --assess --type execute --verbose=4`, then attach the signed/notarized `.app.zip` and `.dmg` to the GitHub Release.
5. Delete the temporary keychain and key material at the end of the job, including on failure.

A release must not be described as signed/notarized unless both signature verification and notarization validation pass.
