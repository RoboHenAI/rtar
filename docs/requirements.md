# TAR Manager AI Implementation Plan (ChatGPT 4.1 Edition)

This document is designed for ChatGPT 4.1 to autonomously implement the TAR Manager in Rust. Follow each step in order. Do not proceed to the next step until the current step is fully implemented and tested. Always reference `features.md` for precise logic, diagrams, and edge cases. If you encounter ambiguity or a missing requirement, STOP and request clarification from the user before continuing.

- Each step is atomic and testable.
- Use imperative language and explicit checks.
- After every milestone, run and document unit/integration tests.
- All logic and diagrams must match those in `features.md`.
- Do not skip steps or combine them unless explicitly instructed.

---

## Milestone 1: TAR Format Foundation

### Step 1.1: Enforce 512-byte Alignment
- **Action:** Implement utilities to ensure all headers and data blocks are always 512 bytes aligned.
- **Acceptance:** All file writes/reads operate on 512-byte boundaries. Unit tests verify alignment.
- **Rationale:** Required by TAR standard; foundation for all operations.

### Step 1.2: Write End-of-Archive Markers
- **Action:** Always finalize the archive with two 512-byte zero blocks.
- **Acceptance:** All archives end with correct markers; extraction with standard tools succeeds.
- **Dependency:** Step 1.1

### Step 1.3: PAX Key-Value Metadata Support
- **Action:** Implement PAX header creation/parsing for arbitrary key-value pairs. All custom metadata (e.g., ROBOHEN_*) must use PAX.
- **Acceptance:** PAX headers are correctly read/written; unknown fields ignored by standard tools.
- **Dependency:** Step 1.1

### Step 1.4: Reserved Field Safety
- **Action:** Ensure no custom fields are written to reserved TAR header fields.
- **Acceptance:** Fuzz/negative tests show only standard fields used outside PAX.
- **Dependency:** Step 1.3

### Step 1.5: PAX Placement
- **Action:** Always write PAX headers immediately before each file entry header, with no gaps.
- **Acceptance:** All file entries with PAX have correct ordering; verified by inspection and test.
- **Dependency:** Step 1.3

---

## Milestone 2: Partitioning Core

### Step 2.1: Partition Trigger
- **Action:** When a file exceeds the partition size limit (default 4KB), split it into partitions.
- **Acceptance:** Files >4KB are partitioned; ≤4KB are not. Unit tests verify logic.
- **Dependency:** Milestone 1

### Step 2.2: First Partition Structure
- **Action:** For the first partition, at the end of the TAR, write:
    1. Two new PAX headers (first: file name, second: ROBOHEN_* metadata)
    2. All original PAX and file entry headers
    3. First N bytes of file data (N=1024 for >4KB, else full file)
- **Acceptance:** Archive structure matches features.md diagrams and logic; verified by test and inspection.
- **Dependency:** Step 2.1, Step 1.5

### Step 2.3: Subsequent Partition Structure
- **Action:** For each new partition, write two PAX headers (file name, ROBOHEN_*) and partition data. Only file name and ROBOHEN_* fields are meaningful in these headers.
- **Acceptance:** All partitions after the first follow this structure.
- **Dependency:** Step 2.2

### Step 2.4: Partition Naming and Collision Handling
- **Action:** Name partitions as `filename.partN`. On collision, append a suffix (`.a`, `.b`, ..., `.aa`, etc.), tracked in ROBOHEN_PART_SUFFIX in the first partition.
- **Acceptance:** No partition name collisions occur; suffixes are correctly tracked and updated.
- **Dependency:** Step 2.3

### Step 2.5: POSIX File Name Enforcement
- **Action:** Validate all file and partition names for POSIX compliance.
- **Acceptance:** Invalid names are rejected or sanitized.
- **Dependency:** Step 2.4

### Step 2.6: Metadata Consistency
- **Action:** All ROBOHEN_* fields are documented, written, and parsed consistently.
- **Acceptance:** Metadata round-trips and is correct for all partitions.
- **Dependency:** Step 2.3

### Step 2.7: Soft-Delete Old Headers
- **Action:** When partitioning, move original file to end of TAR and zero out old headers.
- **Acceptance:** No duplicate logical entries; old headers are zeroed.
- **Dependency:** Step 2.2

### Step 2.8: Remove/Truncate Partitions
- **Action:** When truncating/overwriting, zero out removed partition headers and update the partition chain.
- **Acceptance:** Partition chain and archive state are correct after removal.
- **Dependency:** Step 2.7

### Step 2.9: File Rename in PAX
- **Action:** Perform file renaming in the first new PAX extension header of each affected partition.
- **Acceptance:** Renames are reflected in PAX and on extraction.
- **Dependency:** Step 2.4

---

## Milestone 3: Partition Chain Navigation

### Step 3.1: Metadata Offsets
- **Action:** Each partition must have ROBOHEN_NEXT_PART_OFFSET and ROBOHEN_PREV_PART_OFFSET for bidirectional navigation.
- **Acceptance:** All partitions can be traversed in both directions using these fields.
- **Dependency:** Milestone 2

### Step 3.2: Navigation Logic
- **Action:** Implement logic to navigate through partitions using metadata offsets.
- **Acceptance:** Logical file can be reconstructed from partitions in order.
- **Dependency:** Step 3.1

---

## Milestone 4: Indexing and Read-Only Behavior

### Step 4.1: Index File Creation
- **Action:** Create index file only before first write operation if not present.
- **Acceptance:** Index is created on first write; not on read-only open.
- **Dependency:** Milestone 3

### Step 4.2: Read-Only Mode
- **Action:** Opening a TAR file in read-only mode must never modify the archive or add an index file.
- **Acceptance:** Read-only operations do not trigger writes or index creation.
- **Dependency:** Step 4.1

### Step 4.3: Read-Only Operations
- **Action:** All read-only operations (listing, reading files/chunks/streams) must not trigger any changes.
- **Acceptance:** Archive is unchanged after any read-only operation.
- **Dependency:** Step 4.2

---

## Milestone 5: File Listing

### Step 5.1: Raw File Entry Listing
- **Action:** Provide a function to list all raw file entries (including partitions).
- **Acceptance:** All physical entries are listed, matching archive state.
- **Dependency:** Milestone 4

### Step 5.2: Logical File Listing
- **Action:** Provide a function to list logical files, grouping all partitions as a single logical file.
- **Acceptance:** Logical files are listed, partitions grouped correctly.
- **Dependency:** Step 5.1

---

## Milestone 6: Testing and Compatibility

### Step 6.1: Standard Tool Extraction
- **Action:** Regularly test TAR archives with standard tools (`tar`, `bsdtar`, etc.) for extraction and listing.
- **Acceptance:** Archives are extractable and listable by standard tools.
- **Dependency:** All previous milestones

### Step 6.2: Extra PAX/Partition Tolerance
- **Action:** Ensure extra PAX headers or partitioned files do not prevent extraction or listing by standard tools.
- **Acceptance:** No compatibility issues found in cross-tool tests.
- **Dependency:** Step 6.1

---

**Implementation Notes:**
- At each step, reference the diagrams and logic in features.md.
- After each milestone, run and document unit/integration tests.
- If ambiguity arises, halt and request clarification before proceeding.
