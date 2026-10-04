<#-- Lists the realm's password policy as helper text under a password field.
     Rendered from `passwordPolicies`, so it follows whatever is configured in
     the Keycloak admin console (Authentication → Policies → Password policy). -->
<#macro list>
    <#assign rules = []>
    <#if passwordPolicies.length??><#assign rules += [msg("passwordRequirementLength", passwordPolicies.length)]></#if>
    <#if passwordPolicies.maxLength??><#assign rules += [msg("passwordRequirementMaxLength", passwordPolicies.maxLength)]></#if>
    <#if passwordPolicies.upperCase??><#assign rules += [msg("passwordRequirementUpperCase", passwordPolicies.upperCase)]></#if>
    <#if passwordPolicies.lowerCase??><#assign rules += [msg("passwordRequirementLowerCase", passwordPolicies.lowerCase)]></#if>
    <#if passwordPolicies.digits??><#assign rules += [msg("passwordRequirementDigits", passwordPolicies.digits)]></#if>
    <#if passwordPolicies.specialChars??><#assign rules += [msg("passwordRequirementSpecialChars", passwordPolicies.specialChars)]></#if>
    <#if passwordPolicies.notUsername><#assign rules += [msg("passwordRequirementNotUsername")]></#if>
    <#if passwordPolicies.notEmail><#assign rules += [msg("passwordRequirementNotEmail")]></#if>
    <#if passwordPolicies.passwordHistory??><#assign rules += [msg("passwordRequirementHistory", passwordPolicies.passwordHistory)]></#if>

    <#if rules?has_content>
        <div class="${properties.kcInputHelperTextItemClass} kc-password-requirements" id="password-requirements">
            <span class="${properties.kcInputHelperTextItemTextClass}">
                ${msg("passwordRequirementsTitle")}
                <ul>
                    <#list rules as rule>
                        <li>${rule}</li>
                    </#list>
                </ul>
            </span>
        </div>
    </#if>
</#macro>
