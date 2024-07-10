import { Member, MemberWithRoles, Role, RoleMember } from "@/common/types";
import { useMutation, useQuery } from "@tanstack/react-query";
import axios from "axios";
import { downloadCsv, getDateAsString } from "./utils";

export enum QueryKey {
  MEMBERS_WITH_ROLES = "members_with_roles",
  MEMBERS_WITH_IDS = "members_with_ids",
  MEMBER = "member",
  MEMBER_ROLES = "member_roles",
  ROLES = "roles",
}

export const axios_client = axios.create({
  baseURL: "http://localhost:80/api",
  headers: {
    "Content-Type": "application/json",
  },
});

interface PaginatedQueryParams {
  pageSize: number;
  offset: number;
  search: string;
  sorting: string;
  sort_desc: boolean;
  customFilters: {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  };
}

export function useGetAllMembersWithRoles(params: PaginatedQueryParams) {
  return useQuery<MemberWithRoles[]>({
    queryKey: [QueryKey.MEMBERS_WITH_ROLES, params],
    queryFn: async () => {
      console.log(params);
      const response = await axios_client.get("/members/roles", {
        params: {
          ...params,
          roles: params.customFilters?.roles?.join(",") || undefined,
          valid_from:
            getDateAsString(params.customFilters?.valid_from) || undefined,
          valid_until:
            getDateAsString(params.customFilters?.valid_until) || undefined,
          page_size: params.pageSize,
          customFilters: undefined,
        },
      });
      console.log(response);
      return response.data;
    },
  });
}

export function useExportMembersWithRoles() {
  return useMutation({
    mutationFn: async (params: PaginatedQueryParams) => {
      const response = await axios_client.post("/members/roles/export", null, {
        params: {
          ...params,
          roles: params.customFilters?.roles?.join(",") || undefined,
          valid_from:
            getDateAsString(params.customFilters?.valid_from) || undefined,
          valid_until:
            getDateAsString(params.customFilters?.valid_until) || undefined,
        },
        responseType: "blob",
      });
      return downloadCsv(response.data, `prodeko_members_${new Date().toISOString()}.csv`);
    },
  });
}

export function useGetMembersWithIds(ids: string[]) {
  return useQuery<MemberWithRoles[]>({
    queryKey: [QueryKey.MEMBERS_WITH_IDS, ids],
    queryFn: async () => {
      const response = await axios_client.get("/members", {
        params: {
          user_ids: ids.join(","),
        },
      });
      return response.data;
    },
  });
}

export const useGetMember = (id: string) => {
  return useQuery<MemberWithRoles>({
    queryKey: [QueryKey.MEMBER, { id }],
    queryFn: async () => {
      const response = await axios_client.get(`/members/${id}`);
      return response.data;
    },
  });
};

export const useGetMemberRoles = (id: string) => {
  return useQuery<RoleMember[]>({
    queryKey: [QueryKey.MEMBER_ROLES, { id }],
    queryFn: async () => {
      const response = await axios_client.get(`/members/${id}/roles`);
      return response.data.map((role: RoleMember) => ({
        ...role,
        valid_from: new Date(role.valid_from),
        valid_until: new Date(role.valid_until),
      }));
    },
  });
}

export const useGetRoles = () => {
  return useQuery<Role[]>({
    queryKey: [QueryKey.ROLES],
    queryFn: async () => {
      const response = await axios_client.get("/roles");
      return response.data as Role[];
    },
  });
};

export const useDeleteMember = () => {
  return useMutation({
    mutationFn: async (id: string) => {
      await axios_client.delete(`/members/${id}`);
    },
  });
};

export const useDeleteManyMembers = () => {
  return useMutation({
    mutationFn: async (ids: string[]) => {
      await axios_client.delete(`/members`, {
        data: { ids },
      });
    },
  });
};

export const useAddMultipleRolesToMembers = () => {
  return useMutation<
    void,
    Error,
    {
      userIds: string[];
      roleNames: string[];
      validFrom: Date;
      validUntil: Date;
    }
  >({
    mutationFn: async ({ userIds, roleNames, validFrom, validUntil }) => {
      await axios_client.post("/members/roles", {
        user_ids: userIds,
        role_names: roleNames,
        valid_from: getDateAsString(validFrom),
        valid_until: getDateAsString(validUntil),
      });
    },
  });
};

export const useCreateRole = () => {
  return useMutation<
    void,
    Error,
    {
      name: string;
    }
  >({
    mutationFn: async ({ name }) => {
      await axios_client.post("/roles", {
        name,
      });
    },
  });
}

export const useCreateTargetableRole = () => {
  return useMutation<
    void,
    Error,
    {
      name: string;
      target: string;
    }
  >({
    mutationFn: async ({ name, target }) => {
      await axios_client.post("/application/targetable-roles", {
        name,
        target,
      });
    },
  });
}