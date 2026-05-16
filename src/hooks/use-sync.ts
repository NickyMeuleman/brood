import { useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "../bindings";

export function useSync() {
	const queryClient = useQueryClient();

	// not a useMutation because it's a 1 time sync
	return useQuery({
		queryKey: ["sync"],
		queryFn: async () => {
			const res = await commands.sync();
			if (res.status === "error") {
				throw res.error;
			}

			queryClient.invalidateQueries({ queryKey: ["holdings"] });

			return res.data;
		},
		staleTime: Infinity,
		retry: false,
	});
}
