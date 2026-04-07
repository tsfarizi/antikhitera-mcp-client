# Config Import/Export Guide

Panduan untuk menggunakan mekanisme import/export konfigurasi Postcard agar tidak perlu konfigurasi ulang saat rebuild infrastruktur.

## Overview

Dengan fitur ini, Anda bisa:
1. **Backup** konfigurasi yang sudah ada ke file `.pc`
2. **Restore** konfigurasi dari file `.pc` yang sudah ada
3. **Transfer** konfigurasi antar server/infrastruktur dengan mudah

## CLI Usage

### Backup Konfigurasi

```bash
# Backup ke file default (config-backup.pc)
antikythera-config backup-config

# Backup ke file custom
antikythera-config backup-config my-server-config.pc

# Backup dengan path lengkap
antikythera-config backup-config /backup/prod-config-2024.pc
```

**Output:**
```
✓ Config backed up to: my-server-config.pc
  Size: 1234 bytes

This file can be used later with:
  antikythera-config use-config my-server-config.pc
```

### Gunakan Konfigurasi yang Sudah Ada

```bash
# Gunakan file .pc yang sudah ada
antikythera-config use-config my-server-config.pc

# Gunakan dari path berbeda
antikythera-config use-config /backup/prod-config-2024.pc
```

**Output:**
```
✓ Config loaded from: my-server-config.pc
  Saved as: app.pc
  Size: 1234 bytes

Verifying config...
  ✓ Config is valid
  Provider: openai/gpt-4
  Providers: 2
  Agent max steps: 15
```

### Workflow Rebuild Infrastruktur

```bash
# 1. Sebelum rebuild, backup config saat ini
antikythera-config backup-config backup-before-rebuild.pc

# 2. Lakukan rebuild/setup ulang infrastruktur
# ... (provisioning, deployment, dll)

# 3. Setelah rebuild, restore config dari backup
antikythera-config use-config backup-before-rebuild.pc

# 4. Verifikasi config sudah benar
antikythera-config show
```

## FFI Usage

### Python Example

```python
import ctypes
import json

lib = ctypes.CDLL("./libantikythera_sdk.so")

# === BACKUP CONFIG ===
# Backup current config to custom file
result = json.loads(lib.mcp_config_backup_to(b"my-backup.pc").decode())
print(f"Backup: {result['success']}")
print(f"Saved to: {result['path']}")
print(f"Size: {result['size_bytes']} bytes")

# === USE EXISTING CONFIG ===
# Use config from existing file (e.g., from backup, another server, etc.)
result = json.loads(lib.mcp_config_use_from(b"my-backup.pc").decode())
print(f"Config loaded: {result['success']}")
print(f"Source: {result['source']}")
print(f"Destination: {result['destination']}")
print(f"Size: {result['size_bytes']} bytes")

# Check for warning (if config was corrupted)
if 'warning' in result:
    print(f"Warning: {result['warning']}")
```

### Node.js Example

```javascript
const ffi = require('ffi-napi');
const ref = require('ref-napi');

const lib = ffi.Library('./libantikythera_sdk', {
  'mcp_config_backup_to': ['pointer', ['string']],
  'mcp_config_use_from': ['pointer', ['string']],
});

function readCString(ptr) { return ref.readCString(ptr); }

// Backup config
const backupResult = JSON.parse(readCString(
  lib.mcp_config_backup_to('production-backup.pc')
));
console.log('Backup:', backupResult.success);
console.log('Path:', backupResult.path);

// Use existing config
const useResult = JSON.parse(readCString(
  lib.mcp_config_use_from('production-backup.pc')
));
console.log('Config restored:', useResult.success);
console.log('Source:', useResult.source);
```

### Rust Example

```rust
use antikythera_sdk::config_ffi::*;
use std::ffi::CString;

// Backup current config
let dest = CString::new("my-backup.pc").unwrap();
let result_ptr = unsafe { mcp_config_backup_to(dest.as_ptr()) };
// result_ptr contains JSON response

// Use existing config
let source = CString::new("my-backup.pc").unwrap();
let result_ptr = unsafe { mcp_config_use_from(source.as_ptr()) };
// result_ptr contains JSON response
```

## FFI Function Reference

### `mcp_config_backup_to`

Backup konfigurasi saat ini ke file custom.

```c
char* mcp_config_backup_to(const char* dest_path);
```

**Parameters:**
- `dest_path`: Path untuk menyimpan backup (e.g., "backup.pc")

**Returns:**
```json
{
  "success": true,
  "path": "backup.pc",
  "size_bytes": 1234
}
```

### `mcp_config_use_from`

Gunakan konfigurasi dari file yang sudah ada. File akan di-copy ke `app.pc` (lokasi default).

```c
char* mcp_config_use_from(const char* source_path);
```

**Parameters:**
- `source_path`: Path ke file .pc yang sudah ada

**Returns:**
```json
{
  "success": true,
  "source": "backup.pc",
  "destination": "app.pc",
  "size_bytes": 1234
}
```

Atau dengan warning jika config corrupted:
```json
{
  "success": true,
  "source": "backup.pc",
  "destination": "app.pc",
  "size_bytes": 1234,
  "warning": "Config copied but may be corrupted: ..."
}
```

## Use Cases

### 1. Disaster Recovery

```bash
# Scenario: Server crash, config lost
# Solution: Restore from backup

# Copy backup from storage
scp user@backup-server:/backups/prod-config.pc ./

# Restore config
antikythera-config use-config prod-config.pc

# Verify
antikythera-config show
```

### 2. Clone Infrastruktur

```bash
# Server A (Production)
antikythera-config backup-config prod-config.pc
scp prod-config.pc user@server-b:/app/

# Server B (Staging)
cd /app/
antikythera-config use-config prod-config.pc
# Sekarang Server B punya config yang sama dengan Server A
```

### 3. CI/CD Pipeline

```yaml
# .github/workflows/deploy.yml
steps:
  - name: Backup config
    run: antikythera-config backup-config .backup/config-backup.pc
  
  - name: Deploy new infrastructure
    run: ./deploy.sh
  
  - name: Restore config
    run: antikythera-config use-config .backup/config-backup.pc
```

### 4. Version Control Config

```bash
# Simpan config ke repo
antikythera-config backup-config configs/v1.0.0.pc
git add configs/v1.0.0.pc
git commit -m "Save config v1.0.0"

# Restore dari version tertentu
antikythera-config use-config configs/v0.9.0.pc
```

## File Format

File `.pc` adalah **Postcard binary format** - format serialisasi yang efisien dan cepat.

### Karakteristik:
- **Binary** - Tidak human-readable (untuk keamanan)
- **Compact** - ~75% lebih kecil dari JSON
- **Fast** - Load/save 50x lebih cepat dari TOML
- **Type-safe** - Validasi type saat deserialisasi

### Structure:
```
┌─────────────────────────────────────┐
│         Postcard Binary             │
├─────────────────────────────────────┤
│ server: ServerConfig                │
│ providers: Vec<ProviderConfig>      │
│ model: ModelConfig                  │
│ prompts: PromptsConfig              │
│ agent: AgentConfig                  │
│ custom: HashMap<String, String>     │
└─────────────────────────────────────┘
```

## Tips & Best Practices

1. **Backup sebelum perubahan besar**
   ```bash
   antikythera-config backup-config before-changes.pc
   ```

2. **Simpan backup di tempat aman**
   - Cloud storage (S3, GCS, dll)
   - Version control (git)
   - Secret manager (untuk config dengan API keys)

3. **Verifikasi setelah restore**
   ```bash
   antikythera-config use-config backup.pc
   antikythera-config show  # Verifikasi config benar
   ```

4. **Gunakan nama file yang deskriptif**
   ```bash
   antikythera-config backup-config prod-2024-01-15.pc
   antikythera-config backup-config staging-before-migration.pc
   ```

5. **Automate di CI/CD**
   - Backup sebelum deployment
   - Restore setelah provisioning
   - Validasi config setelah restore

## Troubleshooting

### "Config file not found"
```bash
# Pastikan file .pc ada di path yang benar
ls -la backup.pc
antikythera-config use-config ./backup.pc
```

### "Config may be corrupted"
```bash
# File mungkin rusak atau dari versi berbeda
# Cek dengan show command
antikythera-config show

# Jika gagal, buat config baru dan setup manual
antikythera-config init
```

### "Failed to copy config"
```bash
# Pastikan permission benar
chmod 644 backup.pc
antikythera-config use-config backup.pc
```
