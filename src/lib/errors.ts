import type { AppError } from "../bindings";

export function getErrorMessage(error: AppError): string {
	switch (error.type) {
		case "NotFound":
			return error.data;
		case "Validation":
			return error.data;
		case "MissingData":
			return `Not synced yet: ${error.data}`;
		case "Database":
			return `Database: ${error.data}`;
		case "ExternalService":
			return `External service: ${error.data}`;
		case "Internal":
			return `Internal error: ${error.data}`;
		case "Timeout":
			return `Timeout: ${error.data}`;
	}
}
