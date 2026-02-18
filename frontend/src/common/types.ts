// Re-export generated types from backend (via ts-rs)
export type {
  Application,
  ApplicationStatus,
  ApplicationTargetableRole,
  ApplicationWithMember,
  AuditLogEntryWithActor,
  AuthInfo,
  CreateApplicationResponse,
  CreateEmailTemplate,
  EmailTemplate,
  Member,
  MemberWithRoles,
  NewApplication,
  NewMember,
  NewSavedFilter,
  PostTargetableRole,
  Role,
  RoleMember,
  RoleStats,
  SavedFilter,
  UpdateEmailTemplate,
} from "./generated";

// Frontend-only types (no backend equivalent)

export interface DateRange {
  from: Date;
  to: Date | undefined;
}

export interface Preset {
  name: string;
  label: string;
}

export type ApplicationTargetableRolePK = Pick<
  import("./generated").ApplicationTargetableRole,
  "role_name" | "valid_until"
>;

export type ApplicationWithoutId = Omit<
  import("./generated").Application,
  "application_id"
>;
