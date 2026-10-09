# FileNym Phase 1 Persistence

## Purpose

SQLite stores FileNym's local metadata.

It does not store the contents of registered canonical files.

## Initial Logical Schema

### canonical_files

Conceptual columns:

- id
- current_path
- current_filename
- identity_kind
- identity_value
- size
- modified_at
- registered_at
- last_seen_at
- status
- created_at
- updated_at

The exact representation of filesystem identity should not be frozen until identity experiments are complete.

### aliases

Conceptual columns:

- id
- canonical_file_id
- alias_name
- created_at
- updated_at

`canonical_file_id` references `canonical_files.id`.

## Integrity

Deleting an alias must affect only the alias record.

Deleting/removing a canonical file from FileNym is a separate explicit application action.

The persistence layer must not delete the user's physical file.

## Migrations

All schema changes must use migrations.

The application must be able to initialize an empty database deterministically.

Migration history must be committed to source control.

## Repository Boundary

Application and domain code should depend on repository interfaces rather than SQLite queries.

Conceptually:

`CanonicalFileRepository`

may provide operations such as:

- insert
- find by FileNym ID
- list
- update observed state
- remove registration

`AliasRepository`

may provide:

- create
- list for canonical file
- rename
- remove
- detect duplicates

These are conceptual operations, not a required final Rust API.

Avoid designing repository methods around SQL details.

## Transactions

Operations that modify multiple related records should use transactions where atomicity is required.

## Local Database Location

The final database location should use the application's appropriate local application-data directory rather than the repository directory or user's document folder.

Development and tests may use isolated temporary databases.

## Testing

Persistence tests should use isolated test databases.

Tests must verify at minimum:

- canonical file insertion
- alias insertion
- duplicate constraints
- alias rename
- alias deletion
- canonical file state update
- persistence across reopening the database
- migration from an empty database
- foreign key integrity
