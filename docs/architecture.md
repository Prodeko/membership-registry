# Architecture

This backend follows Domain-Driven Design with Clean Architecture (Ports & Adapters). The goal is to keep business logic pure and infrastructure swappable.

## Layers

```
src/
├── domain/                        Pure business logic, no IO
├── application/
│   ├── ports/                     Traits defining external capabilities
│   └── services/                  Use-case orchestration
└── infrastructure/
    ├── adapters/                  Keycloak, SendGrid, template renderer
    ├── http/                      Axum controllers, DTOs, middleware
    └── repositories/              PostgreSQL DAOs
```

Dependencies point inward. Domain depends on nothing. Application depends on domain. Infrastructure depends on application and domain. No reverse imports, ever.


## Domain layer — `src/domain/`

The domain layer contains business types, invariants, and state machines. It has zero IO imports: no sqlx, reqwest, axum, or any external service client.

Entities have identity and behavior. `Person` and `Application` are entities. Value objects have no identity — two instances with the same data are interchangeable. `Email`, `PersonId`, `ApplicationId`, and `RoleName` are value objects implemented as newtypes with validation on construction.

The application state machine is the core domain model:

```rust
impl Application {
    pub fn apply(&self, action: ApplicationAction)
        -> Result<(ApplicationStatus, ApplicationTransition), TransitionError>
}
```

`ApplicationStatus` encodes the lifecycle: `Unpaid → Pending → Approved | Rejected`. The `apply` method exhaustively matches all state/action combinations, returning either a valid transition or a domain error. Terminal states (`Approved`, `Rejected`) reject all actions.

Domain errors are semantic. `TransitionError::AlreadyTerminal` and `TransitionError::InvalidAction` describe business-rule violations, not technical failures.


## Application layer — `src/application/`

### Ports — `application/ports/`

Ports are traits that define what the application needs from the outside world. The application layer owns these traits; infrastructure implements them.

- `AuthPort` — verify tokens, refresh sessions
- `RoleSyncPort` — CRUD roles in the identity provider
- `EmailPort` — send emails
- `MemberRepositoryPort`, `ApplicationCommandPort`, `ApplicationQueryPort`, `RoleRepositoryPort` — persistence
- `TemplateRepositoryPort`, `TemplateRendererPort` — email templating
- `AuditLogRepositoryPort` — audit trail

Each port defines its own error type. `AuthError` has variants like `TokenExpired` and `Unauthorized`. `RepositoryError` has `NotFound`, `AlreadyExists`, `Constraint`, and `Unexpected`. Infrastructure adapters map their internal errors into these port error types.

### Services — `application/services/`

Services orchestrate use cases. They load data via ports, call domain methods, persist results, and trigger side effects. They contain no business logic themselves — that belongs in the domain.

The key pattern is exhaustive matching on domain transitions:

```rust
match application.apply(action)? {
    (new_status, ApplicationTransition::Approved) => {
        self.repo.update_status(id, new_status).await?;
        self.notifications.send_approved(user).await;
    }
    (new_status, ApplicationTransition::Rejected) => { /* ... */ }
    (new_status, ApplicationTransition::PaymentReceived) => { /* ... */ }
}
```

Adding a new transition variant causes a compile error in every service that matches on transitions, preventing missed cases.

Services map domain and port errors into service-level errors. They never leak domain or infrastructure error types to the HTTP layer.


## Infrastructure layer — `src/infrastructure/`

The infrastructure layer contains all IO and framework-specific code, organized into three submodules.

### Adapters — `infrastructure/adapters/`

Adapters implement port traits using real external services.

- `KeycloakAuthAdapter` implements `AuthPort` — JWT validation via JWKS, token refresh
- `KeycloakRoleSyncAdapter` implements `RoleSyncPort` — role CRUD via Keycloak admin API
- `SendGridEmailAdapter` implements `EmailPort` — email delivery via SendGrid v3 API
- `SimpleTemplateRenderer` implements `TemplateRendererPort` — string replacement with HTML sanitization

Each adapter maps its internal errors to port error types. `KeycloakError::Expired` becomes `AuthError::TokenExpired`. `KeycloakError::Unauthorized` becomes `AuthError::Unauthorized`. The mapping happens in `From` impls on the adapter.

Adapters must not contain business logic. They translate between external service APIs and port contracts.

### Repositories — `infrastructure/repositories/`

Repositories use the DAO pattern with sqlx. Each repository has private DAO structs that match database column shapes, with `From` impls to convert between DAOs and domain types.

```rust
struct MemberDAO { user_id: Uuid, email: String, ... }

impl From<MemberDAO> for Person {
    fn from(row: MemberDAO) -> Self {
        Self { id: PersonId(row.user_id), email: Email::new_unchecked(row.email), ... }
    }
}
```

DAO types are `pub(super)` — they never leak outside the repository module. Domain types enter and leave the repository boundary; DAOs exist only inside.

`PostgresRepo` bundles all repositories into a single struct, constructed once in `main.rs`:

```rust
pub struct PostgresRepo {
    pub member: MemberRepo,
    pub application: ApplicationRepo,
    pub role: RoleRepo,
    pub audit_log: AuditLogRepo,
    // ...
}
```

All queries use `sqlx::query_as!` for compile-time verification against the database schema.

Read models (joined queries producing `MemberWithRoles`, `ApplicationWithMember`, `RoleStats`) are kept separate from domain types. They serve the query side without polluting the domain.

### HTTP — `infrastructure/http/`

Controllers are thin. They parse request DTOs, call a service method, map the result to a response DTO, and return it. No business logic, no direct repository or adapter access — only application services.

```rust
async fn get_members(State(state): State<AppState>, Query(q): Query<MembersQuery>)
    -> ApiResult<Json<Vec<MemberDTO>>>
{
    let members = state.member_service.get_members().await?;
    Ok(Json(members.into_iter().map(MemberDTO::from).collect()))
}
```

Routes are organized into three groups:

- `admin/` — requires admin role (members, applications, roles, audit logs, email templates, saved filters)
- `protected/` — requires authentication (account, user applications, member profile)
- `public/` — no auth required (OAuth2 login flow, Stripe webhooks)

DTOs live in `http/dto/` and all end with `DTO`. They derive `ts_rs::TS` with `#[ts(export)]` to generate TypeScript interfaces in `frontend/src/types/`, keeping frontend types in sync.

Middleware handles auth (`check_auth` extracts JWT from cookies, validates via `AuthenticationService`, handles token refresh), permissions (`check_permission` requires admin role), and member access control (`check_member_access` allows own-data or admin access).

`ApiError` maps service errors to HTTP status codes. `ServiceError::NotFound` becomes 404, `ServiceError::AlreadyExists` becomes 400, `TransitionError` becomes 409. Internal details are stripped from responses.


## Three type families

The codebase maintains strict separation between three type families:

- Domain types (`Person`, `Application`, `Role`) — the source of truth for naming and shape
- DAO types (`MemberDAO`, `ApplicationDAO`) — match database columns, private to repositories
- DTO types (`MemberDTO`, `ApplicationDTO`) — match API shape, used only in HTTP layer

Conversions happen at layer boundaries via `From` impls. Domain types are never reshaped to fit API or database needs.


## Error flow

Each layer defines its own error types and maps errors from the layer below. Errors never leak across boundaries.

```
sqlx::Error
  → RepositoryError (NotFound, AlreadyExists, Constraint, Unexpected)
    → ServiceError (maps repo + domain + port errors)
      → ApiError (maps to HTTP status codes)
```

Domain errors (`TransitionError`) flow through service errors into HTTP errors. A `TransitionError::AlreadyTerminal` becomes a `ServiceError::ApplicationAlreadyProcessed` becomes a 409 Conflict.


## Dependency injection

All services receive their dependencies through constructor injection. `Services::new()` in `main.rs` wires everything:

1. Create `PostgresRepo` from the connection pool
2. Create infrastructure adapters (`KeycloakAuthAdapter`, `SendGridEmailAdapter`, etc.)
3. Create application services, passing ports as `Arc<dyn PortTrait>`
4. Bundle into `AppState` for axum

Services are generic over their ports. In production they get Keycloak and SendGrid adapters. In tests they could get mock implementations of the same traits.


## Compile-time safety

- `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` — no panics in production code
- `sqlx::query_as!` — queries verified against the live database schema at compile time
- Exhaustive `match` on domain transitions — compiler catches unhandled cases
- Newtypes prevent mixing up IDs (`PersonId` vs `ApplicationId` vs `RoleName`)


## Change workflow

When adding or modifying features, work from the inside out:

1. Define or update domain types, invariants, and transitions in `src/domain/`
2. Add orchestration in `src/application/services/`, matching on new transitions
3. Add new ports in `src/application/ports/` only if a new external capability is needed
4. Implement adapters in `src/infrastructure/adapters/` and add controllers/DTOs in `src/infrastructure/http/` last
