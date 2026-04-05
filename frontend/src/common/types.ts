// Re-export generated types from backend (via ts-rs)
export type {
  Application,
  ApplicationAction,
  ApplicationStatus,
  ApplicationTargetableRole,
  ApplicationWithMember,
  AuditLogEntryWithActor,
  AuthenticatedUser,
  CreateApplicationRequest,
  CreateApplicationResponse,
  CreateEmailTemplate,
  EmailTemplate,
  EmailTemplateTranslation,
  KeycloakSyncStatusMap,
  MarketingPreferences,
  MarketingPreferencesUpdate,
  Member,
  MemberKeycloakSyncStatus,
  MemberWithRoles,
  NewMember,
  NewSavedFilter,
  PostTargetableRole,
  PublicConfig,
  Role,
  RoleMember,
  RoleStats,
  SavedFilter,
  SubscriptionAction,
  SubscriptionState,
  TagPreference,
  UpdateMember,
  UpdateRole,
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
