const queryKeys = {
	count: {
		all: () => ["count"],
		id: (id: number) => [...queryKeys.count.all(), id],
	},
};

export default queryKeys;
