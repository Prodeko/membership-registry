import { MemberWithRoles, Role } from "@/common/types";
import { useQuery } from "@tanstack/react-query";
import axios from "axios";
import { getDateAsString } from "./utils";

export enum QueryKey {
  MEMBERS = "members",
  MEMBER = "member",
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
  }
}


export function useGetAllMembers(params: PaginatedQueryParams){
  return useQuery<MemberWithRoles[]>({
    queryKey: [QueryKey.MEMBERS, params],
    queryFn: async () => {
      console.log(params);
      const response = await axios_client.get("/members", {
        params: {
          ...params,
          roles: params.customFilters?.roles?.join(",") || undefined,
          valid_from: getDateAsString(params.customFilters?.valid_from) || undefined,
          valid_until: getDateAsString(params.customFilters?.valid_until) || undefined,  
          page_size: params.pageSize,
          customFilters: undefined
        },
      });
      console.log(response)
      return response.data;
    },
  });
}

export const useGetMember = (id: string) => {
  return useQuery({
    queryKey: [QueryKey.MEMBERS, { id }],
    queryFn: async () => {
      const response = await axios_client.get(`/members/${id}`);
      return response.data;
    },
  });
};


export const useGetRoles = () => {
  return useQuery({
    queryKey: [QueryKey.ROLES],
    queryFn: async () => {
      const response = await axios_client.get("/roles");
      return response.data as Role[];
    },
  });
}
