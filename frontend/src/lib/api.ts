import { MemberWithRoles } from "@/common/types";
import { useQuery } from "@tanstack/react-query";
import axios from "axios";

export enum QueryKey {
  MEMBERS = "members",
  MEMBER = "member",
}

export const axios_client = axios.create({
  baseURL: "http://localhost:80/api",
  headers: {
    "Content-Type": "application/json",
  },
});

interface GetAllMembersParams {
  pageSize: number;
  offset: number;
  roles: string[];
  search: string;
  sorting: string;
  sort_desc: boolean;
}

export function useGetAllMembers(params: GetAllMembersParams){
  return useQuery<MemberWithRoles[]>({
    queryKey: [QueryKey.MEMBERS, params],
    queryFn: async () => {
      const response = await axios_client.get("/members", {
        params: {
          ...params,
          page_size: params.pageSize,
        },
      });
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
