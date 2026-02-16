# Audit Logs

Every state-changing operation in the membership registry is recorded in the `audit_log` table. This provides traceability for member management, role assignments, application processing, and authentication events.

## Design

Audit logging happens at the service layer, not the HTTP handler layer. This ensures that actions triggered by non-HTTP sources (e.g. Stripe webhooks) are also captured.

Logging is fire-and-forget: failures are logged via `tracing::error!` but never propagate to the caller. A failed audit write must not break a business operation.

Each entry records:

- `actor_user_id` - who performed the action (NULL for system actions like Stripe webhooks)
- `action` - dot-separated verb, e.g. `member.create`, `application.update_status`
- `entity_type` / `entity_id` - what was acted upon
- `details` - JSONB with context (old/new values, related IDs)
- `created_at` - timestamp

The `actor_user_id` column has no foreign key to `member`, so audit entries survive member deletion.

## Logged actions

- `member.create`, `member.update`, `member.delete`, `member.delete_many`, `member.export_csv`
- `role.create`, `role.delete`
- `role_member.assign`, `role_member.update`, `role_member.delete`
- `application.create`, `application.update_status`, `application.delete`, `application.payment_received`
- `targetable_role.create`, `targetable_role.update`, `targetable_role.delete`
- `auth.login`
- `auth_provider.link`, `auth_provider.unlink`

## API

```rest
GET /api/admin/audit-logs
```

Query parameters (all optional):

- `page_size` / `offset` - pagination (default 50)
- `action` - exact match filter
- `entity_type` - exact match filter
- `entity_id` - exact match filter
- `actor_user_id` - exact match filter
- `search` - ILIKE search across action, entity_type, entity_id, and actor name

Returns `AuditLogEntryWithActor[]`, which includes the actor's name resolved via LEFT JOIN on `member`.

## Frontend

The audit log viewer is at `/logs` in the admin panel. It is a read-only DataTable with server-side pagination and a search field.

## Schema

```sql
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_user_id UUID,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Indexes on `created_at DESC`, `actor_user_id` (partial), `(entity_type, entity_id)`, and `action`.
