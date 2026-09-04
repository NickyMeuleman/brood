import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useBrokers() {
	return useQuery({
		queryKey: queryKeys.brokers,
		queryFn: async () => {
			const res = await commands.getBrokers();
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
	});
}
