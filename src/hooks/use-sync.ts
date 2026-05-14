import { useQuery } from "@tanstack/react-query";
import { commands } from "../bindings";

export function useSync() {
  // not a useMutation because it's a 1 time sync
	return useQuery({
		queryKey: ["sync"],
		queryFn: async () => {
			const res = await commands.sync();
			if (res.status === "error") {
				throw res.error;
			}
			return res.data;
		},
		staleTime: Infinity,
		retry: false,
	});
}
