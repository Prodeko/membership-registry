// Re-export generated types from backend (via ts-rs)
export type {
  Application,
  ApplicationAction,
  ApplicationStatus,
  ApplicationTargetableRole,
  ApplicationWithMember,
  AttributeDefinition,
  AttributeSyncStatus,
  AuditLogEntryWithActor,
  AuthenticatedUser,
  CreateApplicationRequest,
  CreateAttributeDefinition,
  EditableBy,
  MemberAttribute,
  SetMemberAttribute,
  SyncMissingAttributesSummary,
  SyncMissingFailure,
  UpdateAttributeDefinition,
  CreateApplicationResponse,
  CreateEmailTemplate,
  EmailTemplate,
  EmailTemplateTranslation,
  KeycloakSyncStatusMap,
  CreateMarketingTag,
  MarketingPreferences,
  MarketingPreferencesUpdate,
  MarketingTag,
  Member,
  MemberKeycloakSyncStatus,
  MemberWithRoles,
  NewMember,
  NewSavedFilter,
  PostTargetableRole,
  PublicConfig,
  Role,
  RoleGroup,
  RoleGroupMembership,
  RoleMember,
  RoleStats,
  SavedFilter,
  SubscriptionState,
  TagPreferenceUpdate,
  UpdateMarketingTag,
  UpdateMember,
  UpdateRole,
  UserMarketingTag,
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
