# HaruhiCrypt

**Encryption based on the Haruhi Problem (Superpermutations)**

![Haruhi Suzumiya](./resources/haruhi.jpg)

*Because permutations are also cute, and even more so when Haruhi arranges them.*

---

## The Haruhi Problem

The **Haruhi Problem** is a mathematical problem inspired by the anime series *The Melancholy of Haruhi Suzumiya*, which aired in a non-linear order (14 episodes broadcast across different arcs).

### The Question

> If you wanted to watch the 14 episodes of the first season in **every possible order**, what is the shortest string of episodes you would need to watch?

Formally: **What is the shortest string containing all permutations of n symbols?**

### Known Solutions

| n | Permutations | Minimum Length |
|---|--------------|----------------|
| 2 | 2! = 2 | 3 (`121`) |
| 3 | 6 | 9 (`123121321`) |
| 4 | 24 | 33 |
| 5 | 120 | 153 |
| 8 | 40,320 | ~46,233 (lower bound) |

For n > 5, finding the minimal superpermutation is an **open problem** in mathematics.

### Mathematical Context

A **superpermutation** is a string that contains every permutation of n symbols as a substring. For example, for n=2, `121` contains:
- `12` (positions 1-2)
- `21` (positions 2-3)

The lower bound was proven in 2011 by an anonymous poster on 4chan:
```
length >= n! + (n-1)! + (n-2)! + n - 3
```

In October 2018, Robin Houston, Jay Pantone, and Vince Vatter refined the proof, bringing worldwide attention to the problem.

---

## How HaruhiCrypt Uses This Problem

### Algorithm

1. **Superpermutation Generation**: For n=8, we generate a superpermutation containing all 40,320 permutations as substrings. The superpermutation for n=8 has 317,521 bytes.

2. **Permutation Extraction**: All unique 8-element permutations are extracted from sliding windows of the superpermutation. These form our permutation table.

3. **Block Encryption**: Each 8-byte block is reordered according to a permutation selected by:
   ```
   key → SHA-256 → seed
   offset = hash(seed || block_index) mod 40320
   permutation = permutation_table[offset]
   result[permutation[i]] = block[i]  (for encryption)
   result[i] = block[permutation[i]]  (for decryption)
   ```

### Why This Is Unique

Unlike traditional ciphers that use predetermined S-boxes or fixed permutations, HaruhiCrypt derives permutations from a **superpermutation** - a mathematical object specifically tied to the Haruhi Problem. The superpermutation acts as a "permutation reservoir" where:

- Each permutation appears exactly once as an 8-character window
- Adjacent windows share 7 characters (overlapping property)
- The relationship between superpermutation structure and key-derived offsets creates a complex mapping

### Security

Security is based on two computationally hard problems:

1. **Superpermutations**: Constructing minimal superpermutations is difficult for n > 5 (open problem). Our implementation uses a near-minimal superpermutation.

2. **Irreversibility**: Without knowing the key (seed), determining which permutations were used and in what order is computationally infeasible.

### File Format

```
[1 byte: extension length][N bytes: extension][16 bytes: Random IV][encrypted data][32 bytes: HMAC-SHA256]
```

---

## Usage

### GUI Interface

The UI is inspired by MikuMikuBeam's design with a purple/teal theme and animated elements.

1. Run `haruhi-crypt.exe`
2. Enter a key or generate a random one (🎲 Generate)
3. Select an input file (📂 Browse)
4. Select an output folder (required)
5. Press **🔒 ENCRYPT** or **🔓 DECRYPT**
6. Monitor progress via the stats cards and terminal

### UI Features

- **Dual Image System**: Haruhi image changes during encryption (haruhi.jpg → haruhi-1.jpg)
- **Pulse Effect**: Purple pulse overlay during active processing
- **Stats Cards**: Real-time Bytes and Status display
- **Adaptive Terminal**: Auto-adjusting log display
- **Two-Column Layout**: Key/Files section (left), Buttons/Stats section (right)
- **Background Music**: Dual-track loop with real-time volume control
- **Status Icons**: ⚡ Ready, ⚙️ Processing..., ✅ Done!

### Key Example

A key has the format:
```
a1b2c3d4-e5f6g7h8-i9j0k1l2-m3n4o5p6
```

**Save your key** - without it you won't be able to decrypt your files.

---

## Compilation

```bash
# Development
cargo build

# Release (portable exe)
cargo build --release
```

The standalone executable will be at `target/release/haruhi-crypt.exe`.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      HaruhiCrypt                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌──────────────┐    ┌────────────────┐  │
│  │ Superperm   │ -> │ Permutation  │ -> │ Block Cipher   │  │
│  │ Generator   │    │ Extractor    │    │ (enc/dec)      │  │
│  └─────────────┘    └──────────────┘    └────────────────┘  │
│         │                   │                     │            │
│         v                   v                     v            │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              40,320 permutations (n=8)                    │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### UI Design

Inspired by **MikuMikuBeam**, HaruhiCrypt features a cute and functional interface:

- **Color Scheme**: Purple (#9b59b2) for ENCRYPT, Teal (#1abc9c) for DECRYPT
- **Animation States**: Image transitions and pulse effects during processing
- **Two-Column Layout**: Key/Files on left, Buttons/Stats on right
- **Feedback**: Real-time progress bar, stats cards, and terminal logging
- **Background Music**: Auto-plays on startup with volume slider control

```
┌─────────────────────────────────────────────────────────────┐
│                    🔐 HaruhiCrypt                           │
│               ⚡ Ready / ⚙️ Processing...                   │
│                                                             │
│              [Haruhi Image - Adaptive Size]                │
├─────────────────────────────────┬───────────────────────────┤
│  🔑 Key                         │   🔒 ENCRYPT              │
│  [________________] 🎲 Generate │   🔓 DECRYPT              │
│                                 │                           │
│  📁 Files                       ├───────────────────────────┤
│  Input File: [____________] 📂  │   💾 Bytes  │  🔔 Status  │
│  Output:      [____________] 📂  │   1,234     │  Ready      │
│                                 │                           │
│                                 │   🔊 [ Volume Slider ]     │
├─────────────────────────────────┴───────────────────────────┤
│  ████████████░░░░░░░░░░░░░░░░░░░░░░  50%                    │
├─────────────────────────────────────────────────────────────┤
│  📋 Terminal                                               │
│  [09:15:23] File selected: document.pdf                    │
│  [09:15:24] Starting encryption...                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Dependencies

- **egui/eframe**: Portable GUI (OpenGL)
- **sha2**: Hash for key derivation
- **hmac**: HMAC-SHA256 authentication
- **rand**: Random IV generation
- **rfd**: File selection dialog
- **image**: Image loading for UI
- **chrono**: Timestamps for logging
- **rodio**: Background music playback (minimp3 decoder)

---

## Limitations

- **8-byte blocks**: Encryption works with 8-byte blocks
- **Not verified cryptography**: This is an educational/experimental project. For real use, use AES-GCM or similar.
- **Superpermutation generation**: First-time initialization generates the n=8 superpermutation (~317KB)

---

## References

- [Superpermutation - Wikipedia](https://en.wikipedia.org/wiki/Superpermutation)
- [The Minimal Superpermutation Problem - Nathaniel Johnston](http://www.njohnston.ca/2013/04/the-minimal-superpermutation-problem/)
- [Quanta Magazine: Sci-Fi Writer Greg Egan and Anonymous Math Whiz Advance Permutation Problem](https://www.quantamagazine.org/sci-fi-writer-greg-egan-and-anonymous-math-whiz-advance-permutation-problem-20181105/)
- [4chan Post (2011) - Original Lower Bound Proof](https://www.seanbers.com/4chan-proof/)

---

*HaruhiCrypt - Because anime math can protect your files.*
