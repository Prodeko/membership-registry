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
  CreateMarketingTag,
  EmailTemplate,
  EmailTemplateTranslation,
  KeycloakSyncStatusMap,
  MarketingPreferences,
  MarketingPreferencesUpdate,
  MarketingTag,
  UpdateMarketingTag,
  Member,
  MemberWithRoles,
  AttributeDefinition,
  AttributeSyncStatus,
  CreateAttributeDefinition,
  MemberAttribute,
  SetMemberAttribute,
  SyncMissingAttributesSummary,
  UpdateAttributeDefinition,
  NewMember,
  NewSavedFilter,
  PostTargetableRole,
  PutTargetableRole,
  PublicConfig,
  Role,
  RoleGroup,
  RoleGroupMembership,
  RoleMember,
  RoleStats,
  SavedFilter,
  UpdateMember,
  UpdateRole,
} from "@/common/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import axios, { AxiosError, AxiosResponse } from "axios";
import { downloadCsv, getDateAsString } from "./utils";
import { useNavigate } from "react-router";

export enum QueryKey {
  MEMBERS_WITH_ROLES = "members_with_roles",
  MEMBERS_WITH_IDS = "members_with_ids",
  MEMBER = "member",
  MEMBER_ROLES = "member_roles",
  ROLES = "roles",
  ROLE_GROUPS = "role_groups",
  TARGETABLE_ROLES = "targetable_roles",
  APPLICATIONS = "applications",
  OAUTH = "oauth_callback",
  ME = "me",
  SAVED_FILTERS = "saved_filters",
  LOGOUT = "logout",
  AUDIT_LOGS = "audit_logs",
  EMAIL_TEMPLATES = "email_templates",
  PUBLIC_CONFIG = "public_config",
  KEYCLOAK_SYNC_STATUS = "keycloak_sync_status",
  MARKETING_PREFERENCES = "marketing_preferences",
  MARKETING_TAGS = "marketing_tags",
  ATTRIBUTE_DEFINITIONS = "attribute_definitions",
  ATTRIBUTES_SYNC_STATUS = "attributes_sync_status",
  MEMBER_ATTRIBUTES = "member_attributes",
  MY_ATTRIBUTES = "my_attributes",
}

export const axios_client = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL,
  headers: {
    "Content-Type": "application/json",
  },
  withCredentials: true,
});

axios_client.interceptors.response.use(
  (response: AxiosResponse) => response,
  (error: AxiosError) => {
    if (
      error.response?.status === 401 &&
      !error.config?.url?.includes("/auth/callback")
    ) {
      window.location.href = `${import.meta.env.VITE_API_BASE_URL}/auth/login`;
      return new Promise(() => {}); // halt chain during redirect
    }
    return Promise.reject(error);
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
  (response: AxiosResponse) => response,
  (error: AxiosError) => {
    if (error.response?.status === 401) {
      window.location.href = `${import.meta.env.VITE_API_BASE_URL}/auth/login`;
      return new Promise(() => {}); // halt chain during redirect
    }
    return Promise.reject(error);
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

interface MemberCountParams {
  search?: string;
  customFilters?: {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  };
}

export function useGetMembersCount(params: MemberCountParams) {
  return useQuery<{ total: number }>({
    queryKey: [QueryKey.MEMBERS_WITH_ROLES, "count", params],
    queryFn: async () => {
      const response = await admin_axios_client.get("/members/roles/count", {
        params: {
          search: params.search || undefined,
          roles: params.customFilters?.roles?.join(",") || undefined,
          valid_from:
            getDateAsString(params.customFilters?.valid_from) || undefined,
          valid_until:
            getDateAsString(params.customFilters?.valid_until) || undefined,
        },
      });
      return response.data;
    },
  });
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

export function useGetKeycloakSyncStatus() {
  return useQuery<KeycloakSyncStatusMap>({
    queryKey: [QueryKey.KEYCLOAK_SYNC_STATUS],
    queryFn: async () => {
      const response = await admin_axios_client.get<KeycloakSyncStatusMap>(
        "/members/keycloak-sync-status",
      );
      return response.data;
    },
    staleTime: 60_000,
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
      const response = await admin_axios_client.post("/roles/export", null, {
        responseType: "blob",
      });
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
    enabled: ids.length > 0,
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

export const useCleanupExpiredRoles = () => {
  return useMutation<{ synced: number }, Error>({
    mutationFn: async () => {
      const response = await admin_axios_client.post<{ synced: number }>(
        "/roles/cleanup-expired",
      );
      return response.data;
    },
  });
};

export interface KeycloakSyncResponse {
  added: number;
  failed: number;
  removed: number;
  remove_failed: number;
  users_processed: number;
}

export const useSyncMissingKeycloakRoles = () => {
  const queryClient = useQueryClient();
  return useMutation<KeycloakSyncResponse, Error, { removeExpired: boolean }>({
    mutationFn: async ({ removeExpired }) => {
      const response = await admin_axios_client.post<KeycloakSyncResponse>(
        "/members/keycloak-sync",
        { remove_expired: removeExpired },
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.KEYCLOAK_SYNC_STATUS],
      });
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

export const useRemoveMemberRole = () => {
  return useMutation<
    void,
    Error,
    { userId: string; roleName: string; validFrom: string }
  >({
    mutationFn: async ({ userId, roleName, validFrom }) => {
      await admin_axios_client.delete(`/members/${userId}/roles`, {
        params: { role_name: roleName, valid_from: validFrom },
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

export const useUpdateRole = () => {
  const queryClient = useQueryClient();
  return useMutation<Role, Error, { roleName: string; data: UpdateRole }>({
    mutationFn: async ({ roleName, data }) => {
      const response = await admin_axios_client.put<Role>(
        `/roles/${encodeURIComponent(roleName)}`,
        data,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.ROLES] });
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

export const useUpdateTargetableRole = () => {
  return useMutation<void, Error, PutTargetableRole>({
    mutationFn: async (body: PutTargetableRole) => {
      await admin_axios_client.put("/applications/targetable-roles", body);
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
    queryKey: [QueryKey.APPLICATIONS, params],
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
    queryKey: [QueryKey.OAUTH, params],
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
    queryKey: [QueryKey.SAVED_FILTERS, model],
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
      const response = await axios_client.post("/auth/logout");
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

export const useDeleteEmailTemplate = () => {
  return useMutation<void, Error, string>({
    mutationFn: async (name) => {
      await admin_axios_client.delete(
        `/email-templates/${encodeURIComponent(name)}`,
      );
    },
  });
};

export const useGetEmailTemplateTranslations = (name: string) => {
  return useQuery<EmailTemplateTranslation[]>({
    queryKey: [QueryKey.EMAIL_TEMPLATES, name, "translations"],
    queryFn: async () => {
      const response = await admin_axios_client.get<EmailTemplateTranslation[]>(
        `/email-templates/${encodeURIComponent(name)}/translations`,
      );
      return response.data;
    },
    enabled: !!name,
  });
};

export const useUpsertEmailTemplateTranslation = () => {
  return useMutation<
    EmailTemplateTranslation,
    Error,
    { name: string; locale: string; subject: string; body_html: string }
  >({
    mutationFn: async ({ name, locale, ...body }) => {
      const response = await admin_axios_client.put<EmailTemplateTranslation>(
        `/email-templates/${encodeURIComponent(name)}/translations/${encodeURIComponent(locale)}`,
        body,
      );
      return response.data;
    },
  });
};

export const useDeleteEmailTemplateTranslation = () => {
  return useMutation<void, Error, { name: string; locale: string }>({
    mutationFn: async ({ name, locale }) => {
      await admin_axios_client.delete(
        `/email-templates/${encodeURIComponent(name)}/translations/${encodeURIComponent(locale)}`,
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

export const useGetMarketingPreferences = () => {
  return useQuery<MarketingPreferences>({
    queryKey: [QueryKey.MARKETING_PREFERENCES],
    queryFn: async () => {
      const response = await axios_client.get<MarketingPreferences>(
        "/members/me/marketing-preferences",
      );
      return response.data;
    },
  });
};

export const useUpdateMarketingPreferences = () => {
  const queryClient = useQueryClient();
  return useMutation<MarketingPreferences, Error, MarketingPreferencesUpdate>({
    mutationFn: async (body) => {
      const response = await axios_client.post<MarketingPreferences>(
        "/members/me/marketing-preferences",
        body,
      );
      return response.data;
    },
    onSuccess: (data) => {
      queryClient.setQueryData([QueryKey.MARKETING_PREFERENCES], data);
    },
  });
};

export const useGetMarketingTags = () => {
  return useQuery<MarketingTag[]>({
    queryKey: [QueryKey.MARKETING_TAGS],
    queryFn: async () => {
      const response =
        await admin_axios_client.get<MarketingTag[]>("/marketing-tags");
      return response.data;
    },
  });
};

export const useCreateMarketingTag = () => {
  const queryClient = useQueryClient();
  return useMutation<MarketingTag, Error, CreateMarketingTag>({
    mutationFn: async (tag) => {
      const response = await admin_axios_client.post<MarketingTag>(
        "/marketing-tags",
        tag,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.MARKETING_TAGS] });
    },
  });
};

export const useUpdateMarketingTag = () => {
  const queryClient = useQueryClient();
  return useMutation<
    MarketingTag,
    Error,
    { label: string; data: UpdateMarketingTag }
  >({
    mutationFn: async ({ label, data }) => {
      const response = await admin_axios_client.put<MarketingTag>(
        `/marketing-tags/${encodeURIComponent(label)}`,
        data,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.MARKETING_TAGS] });
    },
  });
};

export const useDeleteMarketingTag = () => {
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: async (label) => {
      await admin_axios_client.delete(
        `/marketing-tags/${encodeURIComponent(label)}`,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.MARKETING_TAGS] });
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
      const response = await axios_client.get<PublicConfig>("/config");
      return response.data;
    },
    staleTime: Infinity,
  });
};

// ---------------------------------------------------------------------------
// Role Groups
// ---------------------------------------------------------------------------

export const useGetRoleGroups = (params?: {
  search?: string;
  [key: string]: unknown;
}) => {
  return useQuery<RoleGroup[]>({
    queryKey: [QueryKey.ROLE_GROUPS, params?.search],
    queryFn: async () => {
      const response =
        await admin_axios_client.get<RoleGroup[]>("/role-groups");
      let data = response.data;
      if (params?.search) {
        const q = params.search.toLowerCase();
        data = data.filter((g) => g.name.toLowerCase().includes(q));
      }
      return data;
    },
  });
};

export const useGetRoleGroup = (id: string) => {
  return useQuery<RoleGroup>({
    queryKey: [QueryKey.ROLE_GROUPS, id],
    queryFn: async () => {
      const response = await admin_axios_client.get<RoleGroup>(
        `/role-groups/${id}`,
      );
      return response.data;
    },
  });
};

export const useGetRoleGroupMembers = (groupId: string) => {
  return useQuery<RoleGroupMembership[]>({
    queryKey: [QueryKey.ROLE_GROUPS, groupId, "members"],
    queryFn: async () => {
      const response = await admin_axios_client.get<RoleGroupMembership[]>(
        `/role-groups/${groupId}/members`,
      );
      return response.data;
    },
  });
};

export const useGetMemberRoleGroups = (userId: string) => {
  return useQuery<RoleGroupMembership[]>({
    queryKey: [QueryKey.ROLE_GROUPS, "member", userId],
    queryFn: async () => {
      const response = await admin_axios_client.get<RoleGroupMembership[]>(
        `/members/${userId}/role-groups`,
      );
      return response.data;
    },
  });
};

export const useCreateRoleGroup = () => {
  const queryClient = useQueryClient();
  return useMutation<
    RoleGroup,
    Error,
    { name: string; description?: string; role_names: string[] }
  >({
    mutationFn: async (data) => {
      const response = await admin_axios_client.post<RoleGroup>(
        "/role-groups",
        data,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.ROLE_GROUPS] });
    },
  });
};

export const useUpdateRoleGroup = () => {
  const queryClient = useQueryClient();
  return useMutation<
    RoleGroup,
    Error,
    { id: string; name: string; description?: string }
  >({
    mutationFn: async ({ id, ...data }) => {
      const response = await admin_axios_client.put<RoleGroup>(
        `/role-groups/${id}`,
        data,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.ROLE_GROUPS] });
    },
  });
};

export const useDeleteRoleGroup = () => {
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: async (id) => {
      await admin_axios_client.delete(`/role-groups/${id}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.ROLE_GROUPS] });
    },
  });
};

export const useSetRoleGroupRoles = () => {
  const queryClient = useQueryClient();
  return useMutation<RoleGroup, Error, { id: string; role_names: string[] }>({
    mutationFn: async ({ id, role_names }) => {
      const response = await admin_axios_client.put<RoleGroup>(
        `/role-groups/${id}/roles`,
        { role_names },
      );
      return response.data;
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ROLE_GROUPS, variables.id],
      });
    },
  });
};

export const useAssignRoleGroup = () => {
  const queryClient = useQueryClient();
  return useMutation<
    void,
    Error,
    {
      groupId: string;
      userId: string;
      validFrom?: string;
      validUntil?: string;
    }
  >({
    mutationFn: async ({ groupId, userId, validFrom, validUntil }) => {
      await admin_axios_client.post(`/role-groups/${groupId}/members`, {
        user_id: userId,
        valid_from: validFrom,
        valid_until: validUntil,
      });
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ROLE_GROUPS, variables.groupId, "members"],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ROLE_GROUPS, "member", variables.userId],
      });
    },
  });
};

export const useRemoveRoleGroupAssignment = () => {
  const queryClient = useQueryClient();
  return useMutation<
    void,
    Error,
    { groupId: string; userId: string; validFrom: string }
  >({
    mutationFn: async ({ groupId, userId, validFrom }) => {
      await admin_axios_client.delete(
        `/role-groups/${groupId}/members/${userId}`,
        { params: { valid_from: validFrom } },
      );
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ROLE_GROUPS, variables.groupId, "members"],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ROLE_GROUPS, "member", variables.userId],
      });
    },
  });
};

// ---------------------------------------------------------------------------
// Flexible user attributes
// ---------------------------------------------------------------------------

export const useGetAttributeDefinitions = () => {
  return useQuery<AttributeDefinition[]>({
    queryKey: [QueryKey.ATTRIBUTE_DEFINITIONS],
    queryFn: async () => {
      const response =
        await admin_axios_client.get<AttributeDefinition[]>("/attributes");
      return response.data;
    },
  });
};

export const useGetAttributeDefinition = (
  name: string,
  options?: { enabled?: boolean },
) => {
  return useQuery<AttributeDefinition>({
    queryKey: [QueryKey.ATTRIBUTE_DEFINITIONS, name],
    queryFn: async () => {
      const response = await admin_axios_client.get<AttributeDefinition>(
        `/attributes/${encodeURIComponent(name)}`,
      );
      return response.data;
    },
    enabled: options?.enabled,
  });
};

export const useCreateAttributeDefinition = () => {
  const queryClient = useQueryClient();
  return useMutation<AttributeDefinition, Error, CreateAttributeDefinition>({
    mutationFn: async (body) => {
      const response = await admin_axios_client.post<AttributeDefinition>(
        "/attributes",
        body,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTE_DEFINITIONS],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTES_SYNC_STATUS],
      });
    },
  });
};

export const useUpdateAttributeDefinition = () => {
  const queryClient = useQueryClient();
  return useMutation<
    AttributeDefinition,
    Error,
    { name: string; data: UpdateAttributeDefinition }
  >({
    mutationFn: async ({ name, data }) => {
      const response = await admin_axios_client.put<AttributeDefinition>(
        `/attributes/${encodeURIComponent(name)}`,
        data,
      );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTE_DEFINITIONS],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTES_SYNC_STATUS],
      });
    },
  });
};

export const useDeleteAttributeDefinition = () => {
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: async (name) => {
      await admin_axios_client.delete(
        `/attributes/${encodeURIComponent(name)}`,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTE_DEFINITIONS],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTES_SYNC_STATUS],
      });
    },
  });
};

export const useGetAttributesSyncStatus = () => {
  return useQuery<AttributeSyncStatus>({
    queryKey: [QueryKey.ATTRIBUTES_SYNC_STATUS],
    queryFn: async () => {
      const response = await admin_axios_client.get<AttributeSyncStatus>(
        "/attributes/sync-status",
      );
      return response.data;
    },
  });
};

export const useSyncMissingAttributes = () => {
  const queryClient = useQueryClient();
  return useMutation<SyncMissingAttributesSummary, Error>({
    mutationFn: async () => {
      const response =
        await admin_axios_client.post<SyncMissingAttributesSummary>(
          "/attributes/sync-missing",
        );
      return response.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTES_SYNC_STATUS],
      });
    },
  });
};

export const useGetMemberAttributes = (
  memberId: string,
  options?: { enabled?: boolean },
) => {
  return useQuery<MemberAttribute[]>({
    queryKey: [QueryKey.MEMBER_ATTRIBUTES, memberId],
    queryFn: async () => {
      const response = await admin_axios_client.get<MemberAttribute[]>(
        `/members/${memberId}/attributes`,
      );
      return response.data;
    },
    enabled: options?.enabled,
  });
};

export const useSetMemberAttribute = (memberId: string) => {
  const queryClient = useQueryClient();
  return useMutation<void, Error, { name: string; value: string }>({
    mutationFn: async ({ name, value }) => {
      const body: SetMemberAttribute = { value };
      await admin_axios_client.put(
        `/members/${memberId}/attributes/${encodeURIComponent(name)}`,
        body,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.MEMBER_ATTRIBUTES, memberId],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTES_SYNC_STATUS],
      });
    },
  });
};

export const useDeleteMemberAttribute = (memberId: string) => {
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: async (name) => {
      await admin_axios_client.delete(
        `/members/${memberId}/attributes/${encodeURIComponent(name)}`,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.MEMBER_ATTRIBUTES, memberId],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKey.ATTRIBUTES_SYNC_STATUS],
      });
    },
  });
};

export const useGetMyAttributes = () => {
  return useQuery<MemberAttribute[]>({
    queryKey: [QueryKey.MY_ATTRIBUTES],
    queryFn: async () => {
      const response =
        await axios_client.get<MemberAttribute[]>("/attributes/me");
      return response.data;
    },
  });
};

export const useSetMyAttribute = () => {
  const queryClient = useQueryClient();
  return useMutation<void, Error, { name: string; value: string }>({
    mutationFn: async ({ name, value }) => {
      const body: SetMemberAttribute = { value };
      await axios_client.put(
        `/attributes/me/${encodeURIComponent(name)}`,
        body,
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.MY_ATTRIBUTES] });
    },
  });
};

export const useDeleteMyAttribute = () => {
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: async (name) => {
      await axios_client.delete(`/attributes/me/${encodeURIComponent(name)}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.MY_ATTRIBUTES] });
    },
  });
};

export interface ImportPreviewRow {
  line: number;
  email: string;
  action?: "create" | "update" | "unchanged";
  error?: string | null;
  changes: string[];
}
export interface MemberImportPreview {
  fatal_error?: string | null;
  create_count: number;
  update_count: number;
  unchanged_count: number;
  error_count: number;
  rows: ImportPreviewRow[];
}
export interface ImportResultRow {
  line: number;
  email: string;
  outcome: string;
  detail?: string | null;
  warning?: string | null;
  changes: string[];
}
export interface MemberImportReport {
  fatal_error?: string | null;
  created: number;
  updated: number;
  unchanged: number;
  skipped: number;
  failed: number;
  rows: ImportResultRow[];
}

export interface RoleImportPreviewRow {
  line: number;
  email: string;
  role_name: string;
  action?: "create" | "update" | "unchanged";
  error?: string | null;
  changes: string[];
}
export interface RoleImportPreview {
  fatal_error?: string | null;
  create_count: number;
  update_count: number;
  unchanged_count: number;
  error_count: number;
  rows: RoleImportPreviewRow[];
}
export interface RoleImportResultRow {
  line: number;
  email: string;
  role_name: string;
  outcome: string;
  detail?: string | null;
  changes: string[];
}
export interface RoleImportReport {
  fatal_error?: string | null;
  created: number;
  updated: number;
  unchanged: number;
  skipped: number;
  failed: number;
  rows: RoleImportResultRow[];
}

const multipart = { headers: { "Content-Type": "multipart/form-data" } };

export const usePreviewMemberImport = () =>
  useMutation<MemberImportPreview, Error, File>({
    mutationFn: async (file) => {
      const fd = new FormData();
      fd.append("file", file);
      const res = await admin_axios_client.post<MemberImportPreview>(
        "/import/members/preview",
        fd,
        multipart,
      );
      return res.data;
    },
  });

export const useApplyMemberImport = () => {
  const queryClient = useQueryClient();
  return useMutation<
    MemberImportReport,
    Error,
    { file: File; sendInvites: boolean }
  >({
    mutationFn: async ({ file, sendInvites }) => {
      const fd = new FormData();
      fd.append("file", file);
      const res = await admin_axios_client.post<MemberImportReport>(
        `/import/members?send_invites=${sendInvites}`,
        fd,
        multipart,
      );
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries();
    },
  });
};

export const usePreviewRoleImport = () =>
  useMutation<RoleImportPreview, Error, File>({
    mutationFn: async (file) => {
      const fd = new FormData();
      fd.append("file", file);
      const res = await admin_axios_client.post<RoleImportPreview>(
        "/import/roles/preview",
        fd,
        multipart,
      );
      return res.data;
    },
  });

export const useApplyRoleImport = () => {
  const queryClient = useQueryClient();
  return useMutation<RoleImportReport, Error, File>({
    mutationFn: async (file) => {
      const fd = new FormData();
      fd.append("file", file);
      const res = await admin_axios_client.post<RoleImportReport>(
        "/import/roles",
        fd,
        multipart,
      );
      return res.data;
    },
    onSuccess: () => {
      queryClient.invalidateQueries();
    },
  });
};
