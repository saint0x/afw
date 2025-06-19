# Aria Authentication & Security Model (v2)

This document outlines the proposed authentication, encryption, and update paradigm for the Aria client and its communication with the firmware. This model prioritizes security and a seamless user experience by leveraging native OS credential management.

## 1. Core Principles

-   **Seamless Identity**: A user's identity is established once via a standard login process (e.g., Google Sign-In) on the Aria client app.
-   **Cryptographic Identity**: The login action triggers the creation of a device-specific cryptographic keypair (`Ed25519`), which becomes the user's true "magic number" for all firmware interactions.
-   **Secure Key Storage**: The highly sensitive private key is **never** stored in a plaintext file. It is delegated to the operating system's native, secure credential manager (e.g., macOS Keychain).
-   **Configuration via TOML**: A single, human-readable configuration file (`aria.toml`) stores non-sensitive metadata, including the public key.

## 2. Client-Side Identity Generation: The "Magic Number"

This process is transparent to the user and happens automatically after their first successful login.

-   **Trigger**: User authenticates into the official Aria macOS client application (e.g., via Google Sign-In).
-   **Action**:
    1.  The client generates a new `Ed25519` cryptographic keypair.
    2.  The **private key** is securely stored in the macOS Keychain, programmatically associated with the Aria application. It is never written to a file in the user's home directory.
    3.  The client creates a configuration file at `~/Library/Application Support/Aria/aria.toml`.

-   **Configuration File (`aria.toml`)**: This file contains the public, non-secret parts of the user's identity.

    ```toml
    # Aria CLI Configuration
    # This file is auto-generated and managed by the Aria client.

    [user]
    # User identifier from the authentication provider (e.g., Google).
    email = "developer@example.com"
    id = "1123581321345589" 

    [device]
    # The public key for this device. The corresponding private key is in the macOS Keychain.
    public_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGCyc2V... user@hostname"
    id = "B7D4F5E6-A8C9-4B7A-9F0D-3E1C8A7B6E5F" # A unique identifier for this device installation.
    ```

### Identity Lifecycle Management

It is critical to manage the identity assets through the user session lifecycle:

-   **On Login**: Before generating a new identity, the client application must check if an `aria.toml` file already exists. If it does, it should verify if the `user.id` in the file matches the currently authenticating user. If it does not match (i.e., a different user was previously logged in), the client **must** cleanly wipe the old `aria.toml` and remove the old private key from the Keychain before generating a new identity.
-   **On Logout**: When a user logs out, the client application **must** perform a cleanup action: delete the `aria.toml` file and remove the user's private key from the Keychain. This prevents a new user on the same machine from accidentally using the previous user's credentials.

## 3. The `arc upload` Workflow

The `arc` CLI leverages this native, secure setup to sign and upload bundles.

1.  **Locate Config**: The `arc upload` command reads `~/Library/Application Support/Aria/aria.toml` to get the user's public key and other metadata.
2.  **Retrieve Private Key**: The CLI makes a system call to the macOS Keychain to retrieve the private key associated with the public key found in the config. The OS may prompt the user for permission on first use.
3.  **Sign the Bundle**: `arc` uses the retrieved private key to sign the `blake3` hash of the `.aria` bundle. This signature proves the bundle originates from an authenticated user on a specific, authorized machine.
4.  **Secure Upload**: The CLI establishes a secure gRPC connection (via a Unix socket) to the firmware, authenticating with the signature, and transmits the bundle to the `pkg_store`.
5.  **Firmware Verification**: The firmware uses the public key (sent with the payload) to verify the bundle's signature.

## 4. Key Decisions & Open Questions (Checklist)

This revised paradigm clarifies some questions and introduces new ones.

-   [x] **Cryptographic Suite Selection**:
    -   [x] **Signing Algorithm**: `Ed25519`. It's fast, secure, and widely adopted.
    -   [ ] **Secure Channel**: `TLS 1.3` with mutual authentication remains the standard. We need to define the certificate management strategy.

-   [x] **Key Management**:
    -   [x] **Private Key Storage**: macOS Keychain (or equivalent for other OSes, e.g., Windows Credential Manager, Secret Service API on Linux). This is resolved.
    -   [ ] **Key Rotation/Revocation**: This is now a function of the user's main account. If a user's device is lost, they can de-authorize the public key associated with that device via their main account settings (a web interface). The firmware would need to sync a revocation list.

-   [ ] **Over-the-Air (OTA) Encryption**:
    -   [ ] Is channel encryption (TLS) sufficient, or does the `.aria` bundle itself need to be encrypted with a one-time key? For internal networks, TLS is often sufficient. For uploads over the public internet, bundle encryption is a stronger posture.

-   [ ] **Firmware Identity & Trust**:
    -   [ ] **User Public Key Management**: How does the firmware maintain a list of trusted public keys for users? Does it query a central identity service?
    -   [ ] **Initial Client Trust**: How does the `arc` CLI or Aria client initially trust the firmware it's talking to? (TOFU, pre-packaged certificates).

-   [ ] **Bundle Security at Rest**:
    -   [ ] Should bundles in the `pkg_store` be encrypted? If so, using what key? A key derived from the firmware's own master key?

This document serves as the starting point for a dedicated security architecture review. 