import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useHeldPositions(asOf: string) {
	return useQuery({
		queryKey: queryKeys.heldPositionsAsOf(asOf),
		queryFn: async () => {
			const res = await commands.getHeldPositions(asOf);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		enabled: Boolean(asOf),
	});
}
