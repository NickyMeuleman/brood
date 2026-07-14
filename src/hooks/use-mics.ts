import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { queryKeys } from "@/lib/queryKeys";

export function useMics() {
	return useQuery({
		queryKey: queryKeys.supportedMics,
		queryFn: () => commands.getMics(),
		staleTime: Infinity,
	});
}
