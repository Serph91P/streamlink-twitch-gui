import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
	{ ignores: ["dist", "src-tauri/target"] },
	eslint.configs.recommended,
	...tseslint.configs.recommended,
	reactHooks.configs["flat/recommended"],
	{
		files: ["**/*.{ts,tsx}"],
		languageOptions: {
			parserOptions: {
				projectService: true,
				tsconfigRootDir: import.meta.dirname,
			},
		},
	},
);
