import {
  Application,
  ApplicationStatus,
  ApplicationTargetableRole,
  ApplicationTargetableRolePK,
  ApplicationWithMember,
  AuthInfo,
  CreateApplicationResponse,
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
} from "@/common/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import axios, { AxiosError, AxiosResponse } from "axios";
import { downloadCsv, getDateAsString } from "./utils";
import { useNavigate } from "react-router-dom";

export enum QueryKey {
  MEMBERS_WITH_ROLES = "members_with_roles",
  MEMBERS_WITH_IDS = "members_with_ids",
  MEMBER = "member",
  MEMBER_ROLES = "member_roles",
  ROLES = "roles",
  TARGETABLE_ROLES = "targetable_roles",
  APPLICATIONS = "applications",
  OAUTH = "oauth_callback",
  ME = "me",
  SAVED_FILTERS = "saved_filters",
  LOGOUT = "logout",
}

export const axios_client = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL,
  headers: {
    "Content-Type": "application/json",
  },
  withCredentials: true,
});

axios_client.interceptors.response.use(
  (response: AxiosResponse) => {
    return response;
  },
  (error: AxiosError) => {
    console.error("Axios error: ", error);
    window.location.href = `/error/${error.response?.status || "500"}`;
  }
);

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
      return downloadCsv(
        response.data,
        `prodeko_members_${new Date().toISOString()}.csv`
      );
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
      const response = await axios_client.get<RoleMember[]>(`/members/${id}/roles`);
      return response.data;
    },
  });
};

export const useGetRoles = () => {
  return useQuery<Role[]>({
    queryKey: [QueryKey.ROLES],
    queryFn: async () => {
      const response = await axios_client.get<Role[]>("/roles");
      return response.data;
    },
  });
};

export const useGetRolesStats = (params: PaginatedQueryParams) => {
  return useQuery<RoleStats[]>({
    queryKey: [QueryKey.ROLES, params],
    queryFn: async () => {
      const response = await axios_client.get<RoleStats[]>("/roles/stats", {
        params: {
          ...params,
          page_size: params.pageSize,
          customFilters: undefined,
        },
      });
      return response.data;
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
};

export const useGetTargetableRoles = () => {
  return useQuery<ApplicationTargetableRole[]>({
    queryKey: [QueryKey.TARGETABLE_ROLES],
    queryFn: async () => {
      const response = await axios_client.get<ApplicationTargetableRole[]>("/applications/targetable-roles");
      return response.data;
    },
  });
};

export const useCreateTargetableRole = () => {
  return useMutation<void, Error, PostTargetableRole>({
    mutationFn: async (targetable_role: PostTargetableRole) => {
      await axios_client.post("/applications/targetable-roles", targetable_role);
    },
  });
};

export const useDeleteTargetableRole = () => {
  return useMutation<void, Error, ApplicationTargetableRolePK>({
    mutationFn: async (id: ApplicationTargetableRolePK) => {
      await axios_client.delete(`/applications/targetable-roles`, {
        params: {
          role_name: id.role_name,
          valid_until: id.valid_until,
        },
      });
    },
  });
};

export const useCreateApplication = () => {
  return useMutation<CreateApplicationResponse, Error, NewApplication>({
    mutationFn: async (newApplication) => {
      const data = await axios_client.post("/applications", newApplication);
      return data.data;
    },
  });
};

export const useCreateMember = () => {
  return useMutation<Member, Error, NewMember>({
    mutationFn: async (member) =>
      (await axios_client.post<Member>("/members", member)).data,
  });
};

export const useGetApplications = (params: PaginatedQueryParams) => {
  return useQuery<ApplicationWithMember[]>({
    queryKey: [QueryKey.APPLICATIONS],
    queryFn: async () => {
      const response = await axios_client.get<ApplicationWithMember[]>("/applications/filter", {
        params: {
          ...params,
          status: params.customFilters?.status,
          page_size: params.pageSize,
          customFilters: undefined,
        },
      });
      return response.data;
    },
  });
};

export const useGetUserApplications = () => {
  return useQuery<Application[]>({
    queryKey: [QueryKey.APPLICATIONS],
    queryFn: async () => {
      const response = await axios_client.get<Application[]>(`/applications/user`);
      return response.data;
    },
  });
};

export const useDeleteApplication = () => {
  return useMutation<void, Error, string>({
    mutationFn: async (id: string) => {
      await axios_client.delete(`/applications/${id}`);
    },
  });
};

export const useSetApplicationStatus = () => {
  return useMutation<void, Error, { id: string; status: ApplicationStatus }>({
    mutationFn: async ({ id, status }) => {
      await axios_client.put(`/applications/${id}/status`, { status });
    },
  });
};

export interface OauthCallbackParams {
  code: string;
  state: string;
}
interface CallbackResponse {
  redirect_to: string;
}
export const useOauthCallback = (params: OauthCallbackParams) => {
  return useQuery<CallbackResponse>({
    queryKey: [QueryKey.OAUTH],
    queryFn: async () => {
      const response = await axios_client.get<CallbackResponse>("/auth/callback", {
        params,
      });
      return response.data;
    },
  });
};

export const useGetMeMember = () => {
  return useQuery<Member>({
    queryKey: [QueryKey.ME],
    queryFn: async () => {
      const response = await axios_client.get("/members/me");
      return response.data;
    },
  });
};

export const useGetMeUser = () => {
  return useQuery<AuthInfo>({
    queryKey: [QueryKey.ME],
    queryFn: async () => {
      const response = await axios_client.get("/users/me");
      return response.data;
    },
  });
};

export const useGetSavedFilters = (model: string) => {
  return useQuery<SavedFilter[]>({
    queryKey: [QueryKey.SAVED_FILTERS],
    queryFn: async () => {
      const response = await axios_client.get("/saved-filters", {
        params: {
          model,
        },
      });
      return response.data;
    },
  });
};

export const useCreateSavedFilter = () => {
  return useMutation<void, Error, NewSavedFilter>({
    mutationFn: async (filter) => {
      await axios_client.post("/saved-filters", filter);
    },
  });
};

export const useDeleteSavedFilter = () => {
  return useMutation<void, Error, string>({
    mutationFn: async (name) => {
      await axios_client.delete(`/saved-filters/${name}`);
    },
  });
};

export const useLogout = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const response = await axios_client.get("/auth/logout");
      return response.data;
    },
    onSuccess: () => {
      queryClient.removeQueries();
      navigate(0);
    },
    onError: (error) => {
      console.error("Logout failed", error);
    },
  });
};

