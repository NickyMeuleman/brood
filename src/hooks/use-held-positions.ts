import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useHeldPositions(asOf: string, brokerId?: number) {
	return useQuery({
		queryKey: queryKeys.heldPositionsAsOf(asOf, brokerId),
		queryFn: async () => {
			const res = await commands.getHeldPositions(asOf, brokerId ?? null);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		enabled: Boolean(asOf),
	});
}
