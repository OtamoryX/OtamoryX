export const getApiErrorMessage = (error: unknown, fallback: string): string => {
  if (typeof error !== "object" || error === null) {
    return fallback;
  }

  const maybeError = error as {
    response?: { data?: { message?: unknown } };
    message?: unknown;
  };

  const responseMessage = maybeError.response?.data?.message;
  if (typeof responseMessage === "string" && responseMessage.trim().length > 0) {
    return responseMessage;
  }

  if (typeof maybeError.message === "string" && maybeError.message.trim().length > 0) {
    return maybeError.message;
  }

  return fallback;
};
