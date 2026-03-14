import {
  Application,
  ApplicationAction,
  ApplicationTargetableRole,
  ApplicationTargetableRolePK,
  ApplicationWithMember,
  AuditLogEntryWithActor,
  AuthenticatedUser,
  CreateApplicationRequest,
  CreateApplicationResponse,
  CreateEmailTemplate,
  EmailTemplate,
  Member,
  MemberWithRoles,
  NewMember,
  NewSavedFilter,
  PostTargetableRole,
  PublicConfig,
  Role,
  RoleMember,
  RoleStats,
  SavedFilter,
  UpdateEmailTemplate,
  UpdateMember,
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
  AUDIT_LOGS = "audit_logs",
  EMAIL_TEMPLATES = "email_templates",
  PUBLIC_CONFIG = "public_config",
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
  },
);

export const admin_axios_client = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL + "/admin",
  headers: {
    "Content-Type": "application/json",
  },
  withCredentials: true,
});

admin_axios_client.interceptors.response.use(
  (response: AxiosResponse) => {
    return response;
  },
  (error: AxiosError) => {
    console.error("Axios error: ", error);
    window.location.href = `/error/${error.response?.status || "500"}`;
  },
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
      const response = await admin_axios_client.get("/members/roles", {
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

export function useExportAllMembers() {
  return useMutation({
    mutationFn: async () => {
      const response = await admin_axios_client.post(
        "/members/roles/export",
        null,
        { responseType: "blob" },
      );
      return downloadCsv(
        response.data,
        `members_${new Date().toISOString()}.csv`,
      );
    },
  });
}

export function useExportApplications() {
  return useMutation({
    mutationFn: async () => {
      const response = await admin_axios_client.post(
        "/applications/export",
        null,
        { responseType: "blob" },
      );
      return downloadCsv(
        response.data,
        `applications_${new Date().toISOString()}.csv`,
      );
    },
  });
}

export function useExportAuditLogs() {
  return useMutation({
    mutationFn: async () => {
      const response = await admin_axios_client.post(
        "/audit-logs/export",
        null,
        { responseType: "blob" },
      );
      return downloadCsv(
        response.data,
        `audit_logs_${new Date().toISOString()}.csv`,
      );
    },
  });
}

export function useExportRoles() {
  return useMutation({
    mutationFn: async () => {
      const response = await admin_axios_client.post(
        "/roles/export",
        null,
        { responseType: "blob" },
      );
      return downloadCsv(
        response.data,
        `roles_${new Date().toISOString()}.csv`,
      );
    },
  });
}

export function useGetMembersWithIds(ids: string[]) {
  return useQuery<MemberWithRoles[]>({
    queryKey: [QueryKey.MEMBERS_WITH_IDS, ids],
    queryFn: async () => {
      const response = await admin_axios_client.get("/members", {
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

export const useGetMemberRoles = (
  id: string,
  options?: { enabled?: boolean },
) => {
  return useQuery<RoleMember[]>({
    queryKey: [QueryKey.MEMBER_ROLES, { id }],
    queryFn: async () => {
      const response = await axios_client.get<RoleMember[]>(
        `/members/${id}/roles`,
      );
      return response.data;
    },
    enabled: options?.enabled,
  });
};

export const useGetRoles = () => {
  return useQuery<Role[]>({
    queryKey: [QueryKey.ROLES],
    queryFn: async () => {
      const response = await admin_axios_client.get<Role[]>("/roles");
      return response.data;
    },
  });
};

export const useGetRole = (
  roleName: string,
  options?: { enabled?: boolean },
) => {
  return useQuery<Role>({
    queryKey: [QueryKey.ROLES, roleName],
    queryFn: async () => {
      const response = await admin_axios_client.get<Role>(
        `/roles/${encodeURIComponent(roleName)}`,
      );
      return response.data;
    },
    enabled: options?.enabled,
  });
};

export const useGetRoleMembers = (
  roleName: string,
  options?: { enabled?: boolean },
) => {
  return useQuery<Member[]>({
    queryKey: [QueryKey.ROLES, roleName, "members"],
    queryFn: async () => {
      const response = await admin_axios_client.get<Member[]>(
        `/roles/${encodeURIComponent(roleName)}/members`,
      );
      return response.data;
    },
    enabled: options?.enabled,
  });
};

export const useGetRolesStats = (params: PaginatedQueryParams) => {
  return useQuery<RoleStats[]>({
    queryKey: [QueryKey.ROLES, params],
    queryFn: async () => {
      const response = await admin_axios_client.get<RoleStats[]>(
        "/roles/stats",
        {
          params: {
            ...params,
            page_size: params.pageSize,
            customFilters: undefined,
          },
        },
      );
      return response.data;
    },
  });
};

export const useDeleteMember = () => {
  return useMutation({
    mutationFn: async (id: string) => {
      await admin_axios_client.delete(`/members/${id}`);
    },
  });
};

export const useDeleteManyMembers = () => {
  return useMutation({
    mutationFn: async (ids: string[]) => {
      await admin_axios_client.delete(`/members`, {
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
      await admin_axios_client.post("/members/roles", {
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
      await admin_axios_client.post("/roles", {
        name,
      });
    },
  });
};

export const useGetTargetableRoles = () => {
  return useQuery<ApplicationTargetableRole[]>({
    queryKey: [QueryKey.TARGETABLE_ROLES],
    queryFn: async () => {
      const response = await axios_client.get<ApplicationTargetableRole[]>(
        "/applications/targetable-roles",
      );
      return response.data;
    },
  });
};

export const useCreateTargetableRole = () => {
  return useMutation<void, Error, PostTargetableRole>({
    mutationFn: async (targetable_role: PostTargetableRole) => {
      await admin_axios_client.post(
        "/applications/targetable-roles",
        targetable_role,
      );
    },
  });
};

export const useDeleteTargetableRole = () => {
  return useMutation<void, Error, ApplicationTargetableRolePK>({
    mutationFn: async (id: ApplicationTargetableRolePK) => {
      await admin_axios_client.delete(`/applications/targetable-roles`, {
        params: {
          role_name: id.role_name,
          valid_until: id.valid_until,
        },
      });
    },
  });
};

export const useCreateApplication = () => {
  return useMutation<
    CreateApplicationResponse,
    Error,
    CreateApplicationRequest
  >({
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

export const useGetApplication = (
  id: string,
  options?: { enabled?: boolean },
) => {
  return useQuery<ApplicationWithMember>({
    queryKey: [QueryKey.APPLICATIONS, id],
    queryFn: async () => {
      const response = await admin_axios_client.get<ApplicationWithMember>(
        `/applications/${id}`,
      );
      return response.data;
    },
    enabled: options?.enabled,
  });
};

export const useGetApplications = (params: PaginatedQueryParams) => {
  return useQuery<ApplicationWithMember[]>({
    queryKey: [QueryKey.APPLICATIONS],
    queryFn: async () => {
      const response = await admin_axios_client.get<ApplicationWithMember[]>(
        "/applications/filter",
        {
          params: {
            ...params,
            status: params.customFilters?.status,
            page_size: params.pageSize,
            customFilters: undefined,
          },
        },
      );
      return response.data;
    },
  });
};

export const useGetUserApplications = () => {
  return useQuery<Application[]>({
    queryKey: [QueryKey.APPLICATIONS],
    queryFn: async () => {
      const response =
        await axios_client.get<Application[]>(`/applications/user`);
      return response.data;
    },
  });
};

export const useDeleteApplication = () => {
  return useMutation<void, Error, string>({
    mutationFn: async (id: string) => {
      await admin_axios_client.delete(`/applications/${id}`);
    },
  });
};

export const useSetApplicationStatus = () => {
  return useMutation<void, Error, { id: string; action: ApplicationAction }>({
    mutationFn: async ({ id, action }) => {
      await admin_axios_client.put(`/applications/${id}/status`, { action });
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
      const response = await axios_client.get<CallbackResponse>(
        "/auth/callback",
        {
          params,
        },
      );
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
  return useQuery<AuthenticatedUser>({
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
      const response = await admin_axios_client.get("/saved-filters", {
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
      await admin_axios_client.post("/saved-filters", filter);
    },
  });
};

export const useDeleteSavedFilter = () => {
  return useMutation<void, Error, string>({
    mutationFn: async (name) => {
      await admin_axios_client.delete(`/saved-filters/${name}`);
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

export function useGetAuditLogs(params: PaginatedQueryParams) {
  return useQuery<AuditLogEntryWithActor[]>({
    queryKey: [QueryKey.AUDIT_LOGS, params],
    queryFn: async () => {
      const response = await admin_axios_client.get("/audit-logs", {
        params: {
          page_size: params.pageSize,
          offset: params.offset,
          search: params.search || undefined,
          action: params.customFilters?.action || undefined,
          entity_type: params.customFilters?.entity_type || undefined,
        },
      });
      return response.data;
    },
  });
}

export const useGetEmailTemplates = () => {
  return useQuery<EmailTemplate[]>({
    queryKey: [QueryKey.EMAIL_TEMPLATES],
    queryFn: async () => {
      const response =
        await admin_axios_client.get<EmailTemplate[]>("/email-templates");
      return response.data;
    },
  });
};

export const useCreateEmailTemplate = () => {
  return useMutation<EmailTemplate, Error, CreateEmailTemplate>({
    mutationFn: async (template) => {
      const response = await admin_axios_client.post<EmailTemplate>(
        "/email-templates",
        template,
      );
      return response.data;
    },
  });
};

export const useUpdateEmailTemplate = () => {
  return useMutation<
    EmailTemplate,
    Error,
    { name: string } & UpdateEmailTemplate
  >({
    mutationFn: async ({ name, ...body }) => {
      const response = await admin_axios_client.put<EmailTemplate>(
        `/email-templates/${encodeURIComponent(name)}`,
        body,
      );
      return response.data;
    },
  });
};

export const useDeleteEmailTemplate = () => {
  return useMutation<void, Error, string>({
    mutationFn: async (name) => {
      await admin_axios_client.delete(
        `/email-templates/${encodeURIComponent(name)}`,
      );
    },
  });
};

export const useUpdateMember = () => {
  const queryClient = useQueryClient();
  return useMutation<Member, Error, { userId: string; data: UpdateMember }>({
    mutationFn: async ({ userId, data }) => {
      const response = await axios_client.put<Member>(
        `/members/${userId}`,
        data,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.ME] });
    },
  });
};

export const useWithdrawApplication = () => {
  const queryClient = useQueryClient();
  return useMutation<void, AxiosError, string>({
    mutationFn: async (applicationId: string) => {
      await axios_client.delete(`/applications/${applicationId}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.APPLICATIONS] });
    },
  });
};

export const useGetPublicConfig = () => {
  return useQuery<PublicConfig>({
    queryKey: [QueryKey.PUBLIC_CONFIG],
    queryFn: async () => {
      const response =
        await axios_client.get<PublicConfig>("/config");
      return response.data;
    },
    staleTime: Infinity,
  });
};
