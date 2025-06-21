export interface NewMember {
  first_name: string;
  last_name: string;
  email: string;
  home_municipality: string;
  has_accepted_policies: boolean;
}

export interface Member extends NewMember {
  user_id: string;
  full_name: string;
}

export interface MemberWithRoles extends Member {
  role_names: string[];
}

export interface Role {
  name: string;
  color: string;
}

export interface RoleStats extends Role {
  member_count: number;
  active_member_count: number;
}

export interface RoleMember {
  user_id: string;
  role_name: string;
  valid_from: Date;
  valid_until: Date;
}

export interface DateRange {
  from: Date;
  to: Date | undefined;
}

export interface Preset {
  name: string;
  label: string;
}


export interface ApplicationTargetableRole {
  role_name: string;
  valid_until: Date;
  active: boolean;
  payment_link?: string;
  optional_roles?: string[];
}

export interface ApplicationTargetableRolePK {
  role_name: string;
  valid_until: Date;
}

export interface NewApplication {
  user_id: string;
  application_text: string;
  role_name: string;
  valid_until: Date;
}

export interface Application extends NewApplication {
  application_id: string;
  status: "pending" | "approved" | "rejected";
  timestamp: Date;
  stripe_payment_id?: string;
}

export type ApplicationWithoutId = Omit<Application, "application_id">;

export interface AuthInfo {
  user_id: string;
  access_token: string;
  first_name: string;
  last_name: string;
  email: string;
}


export interface SavedFilter {
  name: string;
  filtered_model: string;
  owner_user_id: string;
  visible_for_all: boolean;
  search?: string;
  sorting_col?: string;
  sorting_desc: boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  custom_filters?: any;
}

export type NewSavedFilter = Omit<SavedFilter, "owner_user_id">;