# Secrets Management

This directory manages encrypted Solana keypairs using [age](https://age-encryption.org/).

> **Note:** Mainnet deployments are always controlled by a multi-sig. This secrets management system is for development, test, and deployer keys only.

## Setup

1. Install age:

   ```bash
   brew install age
   ```

2. Generate your age key:

   ```bash
   mkdir -p ~/.age
   age-keygen -o ~/.age/prism-protocol-dev.txt
   ```

   Back up this key securely (e.g., in 1Password).

3. Add your public key to `recipients.txt`:

   ```bash
   age-keygen -y ~/.age/prism-protocol-dev.txt >> secrets/recipients.txt
   ```
   (The `-y` flag converts/extracts the public key from your private key file.)

   Commit this change.

4. Install the git hooks:
   ```bash
   ./scripts/install-hooks
   ```
   This will symlink our hooks to your local `.git/hooks` directory.

## Team Onboarding

When a new developer joins the team:

1. The new developer:

   - Follows the Setup steps above
   - Creates a PR adding their public key to `recipients.txt`

2. An existing team member:
   - Reviews the PR
   - Runs `./scripts/decrypt-secrets` to decrypt existing secrets
   - Runs `./scripts/encrypt-secrets` to re-encrypt for all recipients (including the new developer)
   - Commits the updated encrypted files
   - Runs `./scripts/shred-secrets` to clean up

This ensures the new developer can access all existing secrets while maintaining security.

## Directory Structure

- `encrypted-keypairs/`: Contains encrypted `.json.enc` files
- `decrypted-keypairs/`: Contains decrypted `.json` files (gitignored)
- `recipients.txt`: List of developer public keys

## Scripts

### encrypt-secrets

Encrypts files from `decrypted-keypairs/` to `encrypted-keypairs/`, creating secure backups.

```bash
./scripts/encrypt-secrets
```

- Verifies each file is a valid Solana keypair
- Encrypts for all developers in `recipients.txt`
- Shows the Solana address for verification
- Uses temp files for safety

### decrypt-secrets

Decrypts files from `encrypted-keypairs/` to `decrypted-keypairs/` for local use.

```bash
./scripts/decrypt-secrets
```

- Verifies each decrypted file is a valid Solana keypair
- Shows the Solana address for verification
- Uses temp files for safety

### shred-secrets

Securely deletes decrypted files after verifying backups.

```bash
./scripts/shred-secrets
```

- Verifies each file has a backup
- Confirms backup matches the decrypted file
- Securely deletes using `shred -u`

### check-secrets

Used by the pre-commit hook to prevent accidental commits of unencrypted secrets.

```bash
./scripts/check-secrets
```

- Checks for unencrypted `.json` files in `secrets/`
- Ensures all files in `encrypted-keypairs/` have `.enc` extension
- Used automatically by git pre-commit hook

### install-hooks

Installs git hooks to prevent accidental commits of unencrypted secrets.

```bash
./scripts/install-hooks
```

- Symlinks `scripts/git-hooks` to `.git/hooks`
- Ensures hooks are up to date with the repository

## Workflow

1. To work with keypairs:

   ```bash
   ./scripts/decrypt-secrets    # Decrypt for local use
   # ... work with keypairs ...
   ./scripts/encrypt-secrets    # Create secure backup
   ./scripts/shred-secrets      # Clean up
   ```

2. To add a new keypair:
   ```bash
   # Generate keypair
   solana-keygen new -o secrets/decrypted-keypairs/my-keypair.json
   # Create secure backup and clean up
   ./scripts/encrypt-secrets
   ./scripts/shred-secrets
   ```

## Security Notes

- Never commit unencrypted keypairs
- Keep your age key secure
- Run `shred-secrets` when done working with keypairs
- Verify addresses match before shredding
- When adding a new team member, ensure secrets are re-encrypted for all recipients
- The pre-commit hook will prevent accidental commits of unencrypted secrets

### Why explicit shred/restore steps?

- **Pre-commit hooks do not shred automatically**: This is by design. Hooks should never delete or modify your files without your explicit action. If you forget to re-encrypt, automatic shredding could cause data loss.

- **No automatic restore after commit**: Git supports a `post-commit` hook, but restoring decrypted secrets automatically would reintroduce sensitive files into your working directory, which is risky and surprising.

- **Explicit is safer**: Always run `./scripts/shred-secrets` yourself after encrypting and before committing. Restore (`decrypt-secrets`) only when you need to work with secrets.

> This workflow is designed for maximum safety and predictability. Mainnet keys are always managed by multi-sig and are never stored in this system.
