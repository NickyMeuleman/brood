import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands } from "../bindings";
import { getErrorMessage } from "../lib/errors";

export const useCount = (countId: number) => {
	const queryClient = useQueryClient();

	const countQuery = useQuery({
		queryKey: ["count", countId],
		queryFn: async () => {
			const res = await commands.getCount(countId);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		select: (data) => data.value,
		retry: false,
	});

	const incrementMutation = useMutation({
		mutationFn: async () => {
			const res = await commands.incrementCount(countId);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["count", countId] });
		},
	});

	return {
		countQuery,
		incrementMutation,
	};
};
