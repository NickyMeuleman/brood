import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useListings() {
	return useQuery({
		queryKey: queryKeys.listings,
		queryFn: async () => {
			const res = await commands.getListings();
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
	});
}
