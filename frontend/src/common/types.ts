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
  payment_link: string;
  optional_roles: string[];
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
}

export type ApplicationWithoutId = Omit<Application, "application_id">;

export interface AuthInfo {
  user_id: string;
  access_token: string;
  first_name: string;
  last_name: string;
  email: string;
}
