CREATE TABLE canonical_files (
    id TEXT NOT NULL PRIMARY KEY CHECK(length(id) > 0),
    path_encoding TEXT NOT NULL CHECK(path_encoding IN ('windows_utf16le_v1','unix_bytes_v1')),
    path_value BLOB NOT NULL CHECK(length(path_value) > 0),
    identity_scheme TEXT,
    identity_value BLOB,
    size_u64_be BLOB NOT NULL CHECK(length(size_u64_be) = 8),
    modified_seconds INTEGER,
    modified_nanoseconds INTEGER,
    registered_seconds INTEGER NOT NULL,
    registered_nanoseconds INTEGER NOT NULL CHECK(registered_nanoseconds BETWEEN 0 AND 999999999),
    observed_seconds INTEGER NOT NULL,
    observed_nanoseconds INTEGER NOT NULL CHECK(observed_nanoseconds BETWEEN 0 AND 999999999),
    status TEXT NOT NULL CHECK(status IN ('available','modified','unavailable','possible_replacement')),
    unavailable_reason TEXT,
    revision INTEGER NOT NULL CHECK(revision >= 0),
    CHECK((identity_scheme IS NULL AND identity_value IS NULL) OR
          (identity_scheme IS NOT NULL AND identity_value IS NOT NULL AND length(CAST(identity_scheme AS BLOB)) > 0 AND length(identity_value) > 0)),
    CHECK((modified_seconds IS NULL AND modified_nanoseconds IS NULL) OR
          (modified_seconds IS NOT NULL AND modified_nanoseconds IS NOT NULL AND modified_nanoseconds BETWEEN 0 AND 999999999)),
    CHECK((status = 'unavailable' AND unavailable_reason IS NOT NULL AND unavailable_reason IN ('not_found','permission_denied','unknown')) OR
          (status <> 'unavailable' AND unavailable_reason IS NULL))
) STRICT;
CREATE TABLE aliases (
    canonical_file_id TEXT NOT NULL REFERENCES canonical_files(id) ON DELETE RESTRICT ON UPDATE RESTRICT,
    id TEXT NOT NULL CHECK(length(id) > 0),
    name TEXT NOT NULL CHECK(length(CAST(name AS BLOB)) BETWEEN 1 AND 255),
    name_key BLOB NOT NULL CHECK(length(name_key) > 0),
    position INTEGER NOT NULL CHECK(position >= 0),
    created_seconds INTEGER NOT NULL,
    created_nanoseconds INTEGER NOT NULL CHECK(created_nanoseconds BETWEEN 0 AND 999999999),
    updated_seconds INTEGER NOT NULL,
    updated_nanoseconds INTEGER NOT NULL CHECK(updated_nanoseconds BETWEEN 0 AND 999999999),
    PRIMARY KEY(canonical_file_id,id),
    UNIQUE(canonical_file_id,name_key),
    UNIQUE(canonical_file_id,position)
) STRICT;
