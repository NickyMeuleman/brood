import { useMutation, useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/lib/queryKeys";
import { useSyncStore } from "@/stores/sync";
import { commands } from "../bindings";

export function useSync() {
	const queryClient = useQueryClient();
	const { setStatus, setSubmittedAt, setError } = useSyncStore();

	return useMutation({
		mutationFn: async () => {
			const res = await commands.sync();
			if (res.status === "error") {
				throw res.error;
			}
			return res.data;
		},
		onMutate: () => {
			setStatus("pending");
			setSubmittedAt(Date.now());
			setError(null);
		},
		onSuccess: () => {
			setStatus("success");
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
		},
		onError: (e) => {
			setStatus("error");
			setError(e);
		},
	});
}
