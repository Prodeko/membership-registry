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
}

export function useGetAllMembers({
  pageSize,
  offset,
  roles,
  search,
}: GetAllMembersParams){
  return useQuery<MemberWithRoles[]>({
    queryKey: [QueryKey.MEMBERS, { pageSize, offset, roles, search }],
    queryFn: async () => {
      const response = await axios_client.get("/members", {
        params: {
          offset,
          page_size: pageSize,
          roles,
          search,
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
