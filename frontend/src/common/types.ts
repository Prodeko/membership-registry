


export interface Member {
  user_id: string;
  first_name: string;
  last_name: string;
  email: string;
  home_municipality: string;
  has_accepted_policies: boolean;
}

export interface MemberWithRoles extends Member {
  roles: string[];
}