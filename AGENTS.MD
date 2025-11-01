# AGENTS

## Pelayanan Publik Kelurahan Cakung Barat

### Tujuan & Peran
- Melayani warga dalam konteks administrasi kelurahan dengan sapaan hangat, panduan prosedur yang jelas, dan ringkasan langkah praktis.
- Agent aktif secara default pada mode `agent>` di antarmuka STDIO maupun JSON-RPC, memanfaatkan MCP tools untuk menyelesaikan tugas end-to-end sebelum mengembalikan jawaban final.

### Alur Kerja Utama
- Titik masuk utama berada di `Agent::run`, yang dipanggil dari `src/lib/application/stdio.rs` dan `src/lib/infrastructure/rpc/server.rs` ketika mode agent aktif.
- System prompt dibangun dari template di `config/client.toml` dan diperkuat oleh instruksi tambahan `ToolRuntime::compose_system_instructions`.
- Setiap putaran percakapan mewajibkan output JSON valid: gunakan `{"action":"call_tool","tool":"...", "input":{...}}` untuk eksekusi berikutnya atau `{"action":"final","response":"..."}` saat jawaban sudah lengkap.
- Agent otomatis menangani deteksi bahasa dan harus membalas dalam bahasa pengguna; tidak ada tool penerjemahan eksternal yang boleh dipanggil.
- Batas interaksi tool adalah 8 langkah per sesi (`AgentOptions::max_steps`). Jika batas tercapai, agent me-return error ke pengguna.

### Instruksi Sistem yang Terkandung
- Tool dapat didaftarkan ulang oleh agent melalui `{"action":"call_tool","tool":"list_tools"}` untuk mendapatkan manifest terkini.
- Jika tidak ada tool tersedia, agent harus mengandalkan jawaban langsung dari model.
- Panduan tambahan dari MCP server (mis. instruksi server `time` atau `certificate`) disisipkan secara dinamis agar agent mengikuti SOP server terkait.
- Hasil eksekusi tool dibungkus sebagai JSON `tool_result` yang kemudian diteruskan kembali ke model dalam putaran berikutnya, bersama petunjuk `"instruction": "..."` agar model tetap merespons dengan format yang benar.

### Tooling yang Dikonsumsi
- Built-in `list_tools`: mengembalikan daftar tool dan metadata yang sudah diperkaya oleh MCP bridge.
- MCP server `time` menyediakan tool `get_current_time`.
  - Deskripsi (`config/client.toml`): wajib dipanggil di awal percakapan untuk menentukan sapaan dan memperoleh jam akurat.
  - Server ini menerima konfigurasi zona `Asia/Jakarta` dan kota `Jakarta`; metadata tambahan diambil dari server saat runtime.
- MCP server `certificate` menyediakan tool `generate_surat_keterangan_domisili` untuk membuat surat domisili berformat PDF berdasarkan data warga.
- Input schema, deskripsi, dan instruksi server diambil secara dinamis melalui `ToolRuntime::build_context`, sehingga perubahan di server akan otomatis tercermin tanpa harus mengubah kode.

### Konfigurasi Model & Provider
- Provider bawaan: `gemini` dengan model default `gemini-2.5-flash`.
- Provider lain yang sudah dikonfigurasi: Ollama (`gemma3:4b`).
- `AgentOptions` memungkinkan override provider, model, dan system prompt; opsi ini dipermukaan melalui CLI `/agent` atau integrasi pemanggil lain.
- Setiap permintaan ke model direkam di log (lihat `AgentOutcome.logs`) untuk membantu audit percakapan.

### Observabilitas & Mode STDIO
- Gunakan perintah `/log` dan `/steps` di STDIO untuk memeriksa log model dan jejak pemanggilan tool terakhir.
- `/agent on|off|toggle` mengatur apakah prompt berikutnya berjalan dalam mode agent atau langsung ke model.
- `/reset` membersihkan `session_id` sehingga percakapan berikutnya tidak mewarisi konteks sebelumnya.
- Output agent berakhir dengan ringkasan jawaban final yang dikirim sebagai pesan manusia biasa setelah siklus tool selesai.

### Penanganan Error
- Tool yang tidak dikenal -> `ToolError::UnknownTool`, diteruskan ke pengguna sebagai informasi bahwa tool belum tersedia.
- Tool tanpa binding server -> `ToolError::UnboundTool`, menunjukkan konfigurasi MCP perlu diperiksa.
- Kegagalan eksekusi (termasuk respons error dari server) -> `ToolError::Execution`, pesan dari server diteruskan bila ada.
- Respons model yang tidak memenuhi format JSON -> `AgentError::InvalidResponse`, dengan pesan ramah pengguna agar mencoba ulang.

### Pengujian & Pemeliharaan
- Unit test agent berada di `src/lib/application/agent/tests.rs`; jalankan dengan `cargo test agent`.
- Saat menambah tool baru:
  1. Tambahkan entri di `config/client.toml` dengan nama, deskripsi, dan server tujuan.
  2. Pastikan MCP server terkait memberikan metadata dan schema yang konsisten.
  3. Sesuaikan dokumentasi SOP pada server agar agent menerima panduan yang benar melalui `server_instructions`.

### Catatan Untuk Pengembang
- Perubahan pada prompt dasar sebaiknya dilakukan melalui `config/client.toml` agar tetap sinkron dengan konfigurasi UI/CLI lainnya.
- Jika menambah mode interaksi baru (selain STDIO atau JSON-RPC), pastikan untuk menginisialisasi `AgentOptions` secara eksplisit dan menyalin pola log/step agar fitur observabilitas tetap tersedia.
