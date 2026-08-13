import { useQuery } from "@tanstack/react-query";
import { commands, type Period } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useHoldings(period: Period) {
	return useQuery({
		queryKey: queryKeys.holdingsByPeriod(period),
		queryFn: async () => {
			const res = await commands.getHoldings(period);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
	});
}
