export interface Member {
  user_id: string;
  first_name: string;
  last_name: string;
  full_name: string;
  email: string;
  home_municipality: string;
  has_accepted_policies: boolean;
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
