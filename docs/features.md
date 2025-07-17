# TAR Manager Features

## Overview
The TAR Manager provides efficient, partitioned, and asynchronous management of TAR archives using a single file handle and an in-memory header cache. It supports adding, reading, and listing files (including very large files split into parts) without ever opening the TAR file more than once.

## Creating a New TAR Archive
- When a new TAR archive is created using this manager:
  - A metadata field called `ROBOHEN_INDEX_OFFSET` (u64) is added by default to the TAR file itself.
  - This field is initially set to `0` as a placeholder.
  - As files and partitions are added, the index file is constructed and appended to the TAR.
  - Right before the TAR file is closed, the `ROBOHEN_INDEX_OFFSET` field is updated to point to the offset of the index file header, reflecting the final structure and size of the TAR (including any closing TAR bytes or end-of-archive markers).
  - This ensures that any process opening the TAR can instantly locate and validate the index for fast lookups.

  #### Diagram: TAR File Creation Process
  ```
  [Step 1: New TAR]
  +-----------------------------+
  | ROBOHEN_INDEX_OFFSET = 0    |
  +-----------------------------+

  [Step 2: Add Files/Partitions]
  +-----------------------------+----------------+-----------------+ ...
  | ROBOHEN_INDEX_OFFSET = 0    | fileA header   | fileA data      | ...
  +-----------------------------+----------------+-----------------+ ...

  [Step 3: Append Index File]
  +-----------------------------+----------------+-----------------+---------------------+---------------------+
  | ROBOHEN_INDEX_OFFSET = 0    | fileA header   | fileA data      | Index File Header   | Index File Data     |
  +-----------------------------+----------------+-----------------+---------------------+---------------------+

  [Step 4: Finalize TAR]
  +-----------------------------+----------------+-----------------+---------------------+---------------------+-------------------+
  | ROBOHEN_INDEX_OFFSET = <idx>| fileA header   | fileA data      | Index File Header   | Index File Data     | TAR End Markers   |
  +-----------------------------+----------------+-----------------+---------------------+---------------------+-------------------+
  ^
  | (ROBOHEN_INDEX_OFFSET updated to point to Index File Header offset)
  ```

## Key Features

- **Index File for Fast Entry Lookup:**

  #### Diagram: Index File Structure
  ```
  [Index File Binary Layout]
  +--------------------------+-------------------+-------------------+ ... +-----------------------+
  | TAR total file size (u64)| Entry offset 1    | Entry offset 2    | ... | 1 (u64)               |
  +--------------------------+-------------------+-------------------+ ... +-----------------------+
  |         8 bytes          |      8 bytes      |      8 bytes      | ... | 8 bytes               |
  ```
  - The first 8 bytes store the TAR's total file size at the time the index was last updated. When a new TAR is created and the index file is written just before closing, this value is set to the exact expected final TAR file size, including the size of the index file itself (with 512-byte padding as per the TAR format) and the closing TAR bytes.
  - Each following 8 bytes is a u64 offset to a file entry (or part) header in the TAR.
  - The index file always contains at least 50 file entry offsets (u64), padded with zeros if fewer than 50 entries exist. This extra space allows for future file additions without immediate repartitioning.
  - The end of valid entries is marked by a single u64 value of 1. Reading the index file stops at the first 1 value, regardless of file size.
  - Zero values in the entry list are treated as soft-deleted entries and ignored during lookups.
  - The index file is always read and written in blocks sized as a multiple of the **Logical Sector Size** for efficiency. The Logical Sector Size is determined by querying the disk where the target TAR file resides (using system APIs or `/sys/block/.../queue/logical_block_size` on Linux, `diskutil` on macOS, or `wmic` on Windows). This typically does **not** require admin/root access, but if it cannot be determined (e.g., on a network filesystem or restricted environment), a default of 512 bytes is used.
  - The buffer size for all TAR file I/O is always set to a multiple of the Logical Sector Size, with a default target buffer size of 4KB (4096 bytes). If 4KB is not a multiple of the Logical Sector Size, it is rounded up to the next multiple. Users can manually set the buffer size as an optional parameter when creating the TAR manager instance or by setting a property on it.
  - The Logical Sector Size and buffer size are cached and used for all future reads and writes for optimal performance and alignment.

  #### Diagram: Metadata Pointer
  ```
  [TAR File Structure]
  +----------------------+----------------------+-------------------+-------------------+ ...
  | ... (other entries)  | ROBOHEN_INDEX_OFFSET | Index File Header | Index File Data   | ...
  +----------------------+----------------------+-------------------+-------------------+ ...
                         ^
                         | (points to offset of Index File Header)
  ```
  - The `ROBOHEN_INDEX_OFFSET` metadata entry points to the offset of the index file header, allowing instant lookup.

  #### Diagram: Partitioned Index File
  ```
  [Partitioned Index File]
  +-------------------+-------------------+-------------------+
  | index.part1       | index.part2       | index.part3       |
  +-------------------+-------------------+-------------------+
  | <= partition size | <= partition size | <= partition size |
  +-------------------+-------------------+-------------------+
  ```
  - If the index file exceeds the partition size, it is split into `.partN` files, each with its own header and offset.

  #### Diagram: Index Validation and Rebuild Logic
  ```
  [Validation on Open]
  if index_file.TAR_total_size == actual_TAR_size:
      use index for lookups
  else:
      rescan TAR
      write new index file (partitioned if needed)
      update ROBOHEN_INDEX_OFFSET
      handle old partitions (update size, delete extras, overwrite as needed)
  ```

  - The index file and its metadata are updated after any add, remove, or append operation that changes the TAR file size.
  - Benefits:
    - Enables instant, random-access lookup of file entries/parts.
    - Detects and recovers from external modifications to the TAR file.
    - Fully integrated with partitioning and metadata logic.

  - The TAR Manager maintains an in-memory cache (index) of all TAR headers, including each entry's file name, offset, and size.
  - **Why?** Scanning the TAR file for every operation would be slow, especially for large or partitioned archives. By caching the headers, the manager can instantly locate any file or partition without rescanning the archive.
  - **How it works:**
    - On load or after any modification (add/remove), the manager scans the TAR file once and builds the header cache.
    - This cache is then used for all subsequent lookups, reads, and writes.
  - **Efficient Logical File Reassembly:**
    - For partitioned files (e.g., `file.bin.part1`, `file.bin.part2`, ...), the manager uses the cache to quickly find and sort all parts.
    - Logical file reads are performed by seeking to each part's offset (using the cache) and reading the data in order, efficiently reconstructing the full file without scanning the entire archive.
  - **Benefits:**
    - Fast random access to any file or chunk.
    - Scales efficiently even for very large archives or deeply partitioned files.
    - Enables advanced features like chunked reads, appends, and logical file abstraction.

- **Single File Handle:**
  - All TAR operations (add, read, list, remove) use a single open file stream, never opening the same file twice.
- **Header Cache:**
  - An in-memory cache of TAR headers (name, offset, size) is maintained for fast lookups and random access.
- **Partitioned File Support:**
  - Large files are split into `.partN` chunks (e.g., `file.bin.part1`, `file.bin.part2`).
  - Each part is a separate TAR entry; the manager transparently reassembles them on read.
- **Efficient Add/Read:**
  - Adding and reading files (including partitioned files) is performed directly on the TAR file using the header cache.
- **No Double-Open:**
  - The TAR file is never opened more than once for any operation.
- **Async API:**
  - All public operations are async and safe for concurrent use.

## Single File Stream and Mutex Handling

### Diagram: Single File Stream with Mutex

```
        +-------------------------------+
        |      TarManager Instance      |
        +-------------------------------+
        |  Arc<Mutex<File>> (tar file)  |
        +-------------------------------+
                   |      ^
     (lock mutex)  |      |  (unlock mutex)
                   v      |
        +-------------------------------+
        |  File Stream (single handle)  |
        +-------------------------------+
                   |
   +---------------+---------------+---------------+
   |               |               |               |
Read Op 1     Write Op 2      Read Op 3      ...
   |               |               |
   +---------------+---------------+
                   |
         (One at a time, via mutex)
                   |
        +-------------------------------+
        |  File Position Managed:       |
        |  - Seek to correct offset     |
        |  - Perform read/write         |
        +-------------------------------+
```

- **Mutex Protection:**
  - All access to the TAR file stream is protected by an async mutex (`tokio::sync::Mutex<File>`), ensuring that only one operation (read or write) can access the file at a time.
- **Position Management:**
  - Immediately after acquiring the mutex lock on the file, the code checks the current file position using `seek()`.
  - If the file's position does not match the required target position, it seeks to the correct offset before performing any read or write.
  - This ensures all operations are performed at the correct location, even with concurrent access patterns.
- **No Double-Open:**
  - The file is never opened more than once; all operations share the same file handle.
- **Benefits:**
  - Prevents race conditions and data corruption.
  - Guarantees correctness for random and sequential access in async contexts.

## Partitioning Behavior

### Diagram: Partitioned File Layout

```
+----------------+----------------+----------------+----------------+
|  file.part1    |   file.part2   |   file.part3   |   ...          |
+----------------+----------------+----------------+----------------+
|   <=7GB data   |   <=7GB data   |   <=7GB data   |   ...          |
+----------------+----------------+----------------+----------------+
```

- Each partition is a separate TAR entry.
- The last partition may be smaller than 7GB (remainder).
- The manager reassembles the logical file by reading all parts in order.

### Diagram: Write Overflow Handling

```
                 Write chunk (C) to partitioned file

[Before Write]
+----------------+----------------+----------------+
|  part1 (7GB)   |  part2 (7GB)   |  part3 (<7GB)  |
+----------------+----------------+----------------+

Suppose you write chunk C starting at offset X in part3:

[If C fits in part3]
+----------------+----------------+----------------+
|  part1 (7GB)   |  part2 (7GB)   |  part3 (<7GB)  |
+----------------+----------------+----------------+
                                 ^ write C here

[If C overflows part3]
+----------------+----------------+----------------+----------------+
|  part1 (7GB)   |  part2 (7GB)   |  part3 (7GB)   |  part4 (rem)   |
+----------------+----------------+----------------+----------------+
                                 ^ write up to 7GB here
                                              ^ overflow to new part
```

- **Partitioning Rule:**
  - By default, any file larger than 7GB will be automatically split into 7GB parts, with the last part containing the remaining data (≤7GB).
  - Partitioning also occurs when appending data to a file that already exists in the TAR archive, regardless of size.
  - The maximum partition size is customizable by the user, but cannot exceed 7GB.

### Index Validation and Auto-Rebuild

- **On Open:**
  - Look for the `ROBOHEN_INDEX_OFFSET` metadata in the TAR.
  - If found:
    - Seek to the index file entry header using the offset.
    - Read the index file and compare the stored TAR total file size (first `u64`) with the actual TAR file size.
    - If the values **match**: the index is valid; use it for fast lookups.
    - If the values **do not match**: the TAR was likely modified externally (e.g., by another tool). The index is invalid and must be rebuilt:
      - Scan the entire TAR to collect all file entry header offsets.
      - Write a new index file (partition if needed), and update the `ROBOHEN_INDEX_OFFSET` metadata.
      - Handle old index file partitions as follows:
        - **If the new index file partition count == old index file partition count:**
          - Update the final index partition file entry header to set the new partition file size.
        - **If the new index file partition count < old index file partition count:**
          - Delete the extra index file partitions by overriding their file entry headers with zeroes.
        - **If the new index file partition count > old index file partition count:**
          - All OK; old partitions are fully overwritten and new partitions are created as needed.
  - If not found:
    - Scan the entire TAR to build the index and write it as above.

- **On File Add/Remove/Append:**
  - The index file and the `ROBOHEN_INDEX_OFFSET` metadata must be updated, including the new TAR total file size.

### Partitioned Write Handling

- **Chunk Writes to Partitioned Files:**
  - If writing a chunk to a partitioned file and `pos + chunk` would overflow the current partition:
    - **If the partition is the last partition in the TAR:**
      - The chunk is appended to this partition up to the 7GB limit.
      - If the chunk exceeds the limit, a new partition is created for the overflow.
      - When the file is closed, the TAR is finalized by writing the end-of-archive blocks.
    - **If the partition is not the last in the TAR:**
      - The chunk segment is written up to the remaining size of the current partition.
      - If more data remains, writing continues at the beginning of the next partition (if it exists), following the same overflow rule.
      - If the next partition would overflow, a new partition is created for the remaining data.
  - This logic ensures that no partition ever exceeds the maximum size, and data is always written sequentially across partitions as needed.

- **Overflow of Non-Partitioned Files:**
  - If a chunk write to a file that is not currently partitioned causes it to exceed the partition size:
    - The file is converted into a partitioned file:
      - The base file name for all partitions is initially set to the original file name (e.g., `file.txt`).
      - Each partition is named sequentially as `file.txt.part1`, `file.txt.part2`, etc.
      - If, when creating a new partition, a file with the intended partition name already exists in the TAR (collision), a random letter is appended as a subextension to the base partition file name (e.g., `file.txt.a`).
      - All partition file entry headers are renamed accordingly (e.g., `file.txt.a.part1`, `file.txt.a.part2`, ...).
      - Whenever a new partition is created, the file name is saved within the first new PAX extension header that is created for that partition, while all `ROBOHEN_*` metadata fields are saved in the second new PAX extension header.
      - File renaming (due to collision or otherwise) is performed within the first new PAX extension header.
      - The subpartition suffix is tracked in the first partition's metadata as `ROBOHEN_PART_SUFFIX` (e.g., `ROBOHEN_PART_SUFFIX = a`).
      - If another collision occurs, the next available letter is used, and if all single letters are exhausted, the suffix is extended (e.g., `aa`, `ab`, ...).
      - This ensures unique partition file names and prevents collisions.
      - The `ROBOHEN_PART_SUFFIX` metadata field always reflects the current subpartition suffix for the set of partitions.

    #### Diagram: Partition Base Name Collision and Suffix Logic

    | Scenario                  | Partition File Names                       | ROBOHEN_PART_SUFFIX |
    |---------------------------|--------------------------------------------|---------------------|
    | No collision              | file.txt.part1, file.txt.part2             | (none)              |
    | Collision (suffix 'a')    | file.txt.a.part1, file.txt.a.part2         | a                   |
    | Collision (suffix 'b')    | file.txt.b.part1, file.txt.b.part2         | b                   |
    | Collision (suffix 'aa')   | file.txt.aa.part1, file.txt.aa.part2       | aa                  |
    | ...                       | ...                                        | ...                 |

    - When a collision occurs, all partition file entry headers are updated to use the new base name with the suffix.
    - The suffix is stored in the `ROBOHEN_PART_SUFFIX` metadata field of the first partition.
    - Suffixes increment as needed to ensure uniqueness.
    - When a collision occurs, the original file's headers are updated to include the new suffix.

    - **Partitioning Logic:**
      - **General Rule:**
        - Whenever a file is partitioned, the first partition will contain all of the original PAX extension headers and file entry headers, preceded by two new PAX extension headers at the beginning (the first for the file name, the second for ROBOHEN_* metadata). The original headers are preserved and used to represent the virtual file entry for the entire logical file.
        - For any subsequent partitions, only the partition file name and the ROBOHEN_* metadata fields from their headers are considered useful; all other header fields in those partitions are ignored for logical file reconstruction or metadata purposes.
        - Always create the first partition at the end of the TAR file. Do not attempt to fit ROBOHEN_* metadata into existing headers—always use new PAX extension headers as described below.

      - **If the file is less than or equal to 4KB:**  
        - Move the entire original file to the end of the TAR archive.
        - The old original file's PAX and file entry headers will be soft deleted (zeroed out) in the archive.
        - When recreating the file, add two new PAX extension headers at the start:
            1. The first PAX header stores the file name (migrated from the original headers).
            2. The second PAX header stores all required `ROBOHEN_*` metadata fields and values.
        - Then append all original PAX extension and file entry headers from the original file, followed by the file data.
        - Proceed with partitioning and metadata storage as above (the move gives a fresh start for headers).
        - The cached file entries will always be updated as required during partitioning.

        #### Diagram: Partitioning (≤4KB)
        ```
        [Original TAR]
        +----------------+------------------+----------------+
        | Old PAX Header | Old File Header  | File Data      |
        +----------------+------------------+----------------+

        [After Move & Soft Delete]
        +----------------+------------------+----------------+
        | Zeroed Blocks  | Zeroed Blocks    |                |
        +----------------+------------------+----------------+
        [Appended at End]
        +----------------+------------------+----------------+----------------+----------------+----------------+
        | PAX: File Name | PAX: ROBOHEN_*   | Old PAX Header | Old File Header| File Data      | (more data...) |
        +----------------+------------------+----------------+----------------+----------------+----------------+
        ```

      - **If the file is larger than 4KB:**  
        - Create a new file at the end of the TAR (this becomes the first partition).
        - Copy the original file's PAX and file entry headers, then add two new PAX extension headers:
            1. The first PAX header stores the file name (migrated from the original headers).
            2. The second PAX header stores all required `ROBOHEN_*` metadata fields and values.
        - Add the first 1024 bytes of the original file content into the new partition. This ensures it is safe to perform the 1024-byte offset in the original file's headers.
        - Modify the original file’s headers so they are offset by 1024 bytes, meaning the file entry header now overrides the first 1024 bytes of the original file content (to make room for the two new PAX headers).
        - At the original file’s location, add two new PAX extension headers: one for the file name, and one for the `ROBOHEN_*` metadata, as the second partition.
        - Continue partitioning the rest of the file as usual.

        #### Diagram: Partitioning (>4KB)
        ```
        [Original TAR]
        +----------------+------------------+---------------------+---------------------+
        | Old PAX Header | Old File Header  | File Data[0:1024]   | File Data[1024:]    |
        +----------------+------------------+---------------------+---------------------+

        [After Partitioning]
        +----------------+------------------+------------------+------------------+------------------+------------------+----------------+
        | PAX: File Name | PAX: ROBOHEN_*   | Old PAX Header   | Old File Header  | File Data[0:1024]| ... (new parts)  | (rest of data) |
        +----------------+------------------+------------------+------------------+------------------+------------------+----------------+
        [Original location]
        +----------------+------------------+----------------+-------------------------------+----------------------+
        | PAX: File Name | PAX: ROBOHEN_*   | Old PAX Header | Second partition File Header  | File Data[1024:]     |
        +----------------+------------------+----------------+-------------------------------+----------------------+
        ```

    - **Partition Chain Navigation:**
      - For every new partition created:
        - The previous partition's `ROBOHEN_NEXT_PART_OFFSET` metadata field is updated to point to the new partition's header offset.
        - The new partition receives a `ROBOHEN_PREV_PART_OFFSET` metadata field, which points to the previous partition's file entry header offset.
      - This allows efficient navigation through partitions in both directions via metadata, without scanning the entire archive.

      **Note:**
      To maintain full TAR standard compatibility, always finalize the TAR archive by writing two 512-byte blocks of zeros at the end (end-of-archive markers). This is required by the TAR format and ensures interoperability with all standard TAR tools.

      #### Diagram: Partition Chain Navigation
      ```
      [Partition 1] <-> [Partition 2] <-> [Partition 3] ...
         |  ROBOHEN_NEXT_PART_OFFSET  |  ROBOHEN_NEXT_PART_OFFSET  |
         |  ROBOHEN_PREV_PART_OFFSET  |  ROBOHEN_PREV_PART_OFFSET  |
      (bidirectional links via metadata)
      ```
  - **Truncating/Overriding to Remove Partitions:**
    - If a file is truncated or overridden to reduce its size such that it loses one or more partitions:
      - The new last partition's `ROBOHEN_NEXT_PART_OFFSET` metadata field is deleted, as there are no further partitions.
      - Any removed partition file entry headers are zeroed out or marked as deleted.
{{ ... }}
- **Naming Convention:**
  - Partitioned files are stored as `filename.part1`, `filename.part2`, etc.
  - The manager reconstructs the logical file by concatenating all parts in order, using the metadata fields for fast navigation.

## Listing Files

### Cached File Entry Headers Structure
- The cache uses an **Indexed hash struct** to efficiently map and access file entries:

  #### Diagram: Cache Structure
  ```
  Virtual File Cache (Indexed Hash)
  +-------------------+-------------------+-------------------+-------------------+
  | "fileA.bin"       | "fileB.txt"       | "fileC.log"       | ...               |
  +-------------------+-------------------+-------------------+-------------------+
          |                   |                   |
          v                   v                   v
  +------------------+   +------------------+   +------------------+
  | partitions:      |   | partitions:      |   | partitions:      |
  | Indexed Hash     |   | Indexed Hash     |   | Indexed Hash     |
  +------------------+   +------------------+   +------------------+
          |                   |                   |
          v                   v                   v
  +-------------------+   +-------------------+   (empty)
  | part1 header      |   | part1 header      |
  | part2 header      |   | part2 header      |
  | ...               |   | ...               |
  +-------------------+   +-------------------+
  ```
  - Each virtual file entry (logical file) is keyed by its file name (from the first partition's `ROBOHEN_FILE_NAME` metadata).
  - Each virtual file entry has a `partitions` property, which is itself an Indexed hash struct.
  - For files without partitions (e.g., `fileC.log`), the `partitions` hash is empty.
  - The `partitions` struct contains all real partition file entry headers for that logical file, indexed by partition number or offset.
  - This enables fast lookup, traversal, and reassembly of partitioned files.

  #### Diagram: Cache Update Triggers
  ```
  [Cache Update Triggers]
  - On any ROBOHEN_* metadata header creation/modification/deletion
  - After a file write is closed
  - After a file partition is created
  - When a file without partitions is converted into a partitioned file
  ```
- **Naming:**
  - Every partitioned file's virtual file entry will have its file name set as the value of the `ROBOHEN_FILE_NAME` metadata header from the first partition.

There are two listing functions:

- **Raw File Entry Listing:**

  #### Diagram: Raw File Entry Listing

  | File Name         | Size  | NEXT              | PREV              |
  |-------------------|-------|-------------------|-------------------|
  | fileA.bin.part1   | 7GB   | fileA.bin.part2   | (none)            |
  | fileA.bin.part2   | 2GB   | (none)            | fileA.bin.part1   |
  | fileB.txt         | 1MB   | (none)            | (none)            |
  | fileC.log.part1   | 7GB   | fileC.log.part2   | (none)            |
  | fileC.log.part2   | 6GB   | (none)            | fileC.log.part1   |

  - All file entries are listed as they physically appear in the TAR, including partitions.
  - Each entry includes its file name, size (stored and returned as a `u64` byte count, but displayed in the diagram as KB/MB/GB for clarity), and references to the next/previous partition by file name (via `ROBOHEN_NEXT_PART_OFFSET` and `ROBOHEN_PREV_PART_OFFSET`), if applicable.
  - This raw list is used as the basis for the file entry list cache, which stores the direct mapping and chaining between partitions for efficient lookups.

- **Virtual File Entry Listing:**

  #### Diagram: Virtual File Entry Listing

  | Virtual File Name | Size  | Parts                                |
  |-------------------|-------|--------------------------------------|
  | fileA.bin         | 9GB   | [fileA.bin.part1, fileA.bin.part2]   |
  | fileB.txt         | 1MB   | []                                   |
  | fileC.log         | 13GB  | [fileC.log.part1, fileC.log.part2]   |

  (Name from ROBOHEN_FILE_NAME)

  - Logical files are grouped from their partitions, using the original file name from the first partition's `ROBOHEN_FILE_NAME`.
  - The virtual file size is the sum of all partition sizes for that file (stored and returned as a `u64` byte count, but displayed in the diagram as KB/MB/GB for clarity).
  - The Parts field references each partition by its RAW file name in order.
  - Partition details are hidden from the user; only the logical file abstraction is presented.
  - Internally, the cache links each partition for fast traversal and size calculation.

## Removal/Overwrite

Removal and overwrite operations are designed to preserve TAR compatibility and support efficient file management:

- **Soft Deletion:**
  - When a file (partitioned or not) is removed, its file entry header(s) are zeroed out or marked as deleted (soft deletion), rather than physically removing data from the TAR. This preserves archive integrity and compatibility.
  - For partitioned files, all partition headers in the chain (as linked by `ROBOHEN_NEXT_PART_OFFSET` and `ROBOHEN_PREV_PART_OFFSET`) are zeroed out.
  - The cache and index are updated to remove references to deleted file entries and their partitions.

- **Overwrite:**
  - Overwriting a file (partitioned or not) will soft-delete the old file headers as above, and append new file or partition headers for the new data at the end of the TAR.
  - If a file is overwritten with a smaller file (causing fewer partitions), the extra partition headers are zeroed out and the new last partition's `ROBOHEN_NEXT_PART_OFFSET` metadata is deleted.
  - If a file is overwritten with a larger file (causing more partitions), new partitions are created as described in the partitioning logic, with collision handling as needed.
  - The cache and index are updated to reflect the new file entry structure.

- **Metadata and Partition Chain Updates:**
  - When partitions are removed or truncated, the partition chain is updated: the new last partition's `ROBOHEN_NEXT_PART_OFFSET` is deleted, and any removed partition headers are zeroed out.
  - All relevant metadata fields (`ROBOHEN_FILE_NAME`, `ROBOHEN_NEXT_PART_OFFSET`, `ROBOHEN_PREV_PART_OFFSET`, `ROBOHEN_PART_SUFFIX`) are updated or deleted as needed to maintain consistency.

- **Cache/Index Consistency:**
  - After any removal or overwrite, the file entry cache and index are rebuilt or updated to ensure accurate, fast lookups and virtual file grouping.

This approach ensures that file removal and overwrite are robust, efficient, and compatible with both the TAR format and the advanced features of this manager.

## Compatibility
- The archive remains compatible with standard TAR tools, but only this manager understands the `.partN` reassembly logic.
- Only this manager will interpret the `ROBOHEN_*` metadata fields used for partition navigation, virtual file grouping, and index management.

---

# Example Usage

All partitioning and internal file management are handled automatically by the TAR manager. Users interact with logical files only; partitioning is transparent.

## CRUD File Operations
- **Create or Overwrite File**
  ```rust
  manager.write_file("data.csv", &data_bytes).await?;
  ```
- **Read Entire File**
  ```rust
  let contents = manager.read_file("data.csv").await?;
  ```
- **Delete File**
  ```rust
  manager.delete_file("data.csv").await?;
  ```
- **Rename File**
  ```rust
  manager.rename_file("old.csv", "new.csv").await?;
  ```

## Stream/Random Write/Append/Truncate
- **Append Data to File**
  ```rust
  manager.append_file("log.txt", &append_bytes).await?;
  ```
- **Write to File at Offset (Random Access/Partial Update)**
  ```rust
  manager.write_file_chunk("data.csv", offset, &chunk_bytes).await?;
  ```
- **Truncate File**
  ```rust
  manager.truncate_file("data.csv", new_length).await?;
  ```

## Stream/Random Read
- **Read Chunk from File**
  ```rust
  let chunk = manager.read_file_chunk("data.csv", offset, length).await?;
  ```
- **Read File as Async Stream**
  ```rust
  let mut stream = manager.stream_file("data.csv").await?;
  while let Some(chunk) = stream.next().await {
      // process chunk
  }
  ```

## File Listing Functions
- **Raw File Entry Listing**
  ```rust
  let raw_entries = manager.list_raw_entries().await?;
  // Returns all physical entries including partitions
  ```
- **Virtual File Entry Listing**
  ```rust
  let logical_files = manager.list_files().await?;
  // Returns logical files, grouping all partitions
  ```

## Index File Creation and Read-Only Behavior
- Opening an existing TAR file that does not contain an index will **not** modify the archive or add an index file unless a write operation occurs (write, append, create, delete, etc.).
- You can perform **read-only operations** (listing, reading files/chunks/streams) on any TAR file without triggering any changes.
- The index file is created automatically **right before** the first write operation if it does not already exist. This is managed by an internal "index file exists" flag, which is checked and cached when the TAR manager instance is created. All write operations are wrapped to check this flag and trigger index creation if needed.
