import { QueryKey } from "@/common/types";
import { useQuery } from "@tanstack/react-query";
import axios from "axios";

export const axios_client = axios.create({
  baseURL: "http://localhost:80/api",
  headers: {
    "Content-Type": "application/json",
  },
});

export function useGetAllMembers() {
  return useQuery({
    queryKey: ["members"],
    queryFn: async () => {
      const response = await axios_client.get("/members");
      return response.data;
    },
  });
}

export const useGetMember = (id: string) => {
  return useQuery({
    queryKey: [QueryKey.MEMBERS],
    queryFn: async () => {
      const response = await axios_client.get(`/members/${id}`);
      return response.data;
    },
  });
};
