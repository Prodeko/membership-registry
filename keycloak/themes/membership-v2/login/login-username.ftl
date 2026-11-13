<#--
  First step of the membership-browser flow (email → password or passkey).
  Overrides base/login-username.ftl to:
    - put "Remember me" and "Forgot password?" on one row (the stock page only
      offers password reset on the second, password step),
    - separate the passkey option with an "or" divider,
    - render registration as a single "No account? Register" line.
  Styles: resources/css/login-extras.css
-->
<#import "template.ftl" as layout>
<#import "passkeys.ftl" as passkeys>
<@layout.registrationLayout displayMessage=!messagesPerField.existsError('username') displayInfo=(realm.password && realm.registrationAllowed && !registrationDisabled??); section>
<!-- template: login-username.ftl (membership-v2) -->
    <#if section = "header">
        ${msg("loginAccountTitle")}
    <#elseif section = "form">
        <div id="kc-form">
            <div id="kc-form-wrapper">
                <#if realm.password>
                    <form id="kc-form-login" onsubmit="login.disabled = true; return true;" action="${url.loginAction}" method="post">
                        <#if !usernameHidden??>
                            <div class="${properties.kcFormGroupClass!}">
                                <label for="username" class="${properties.kcLabelClass!}"><#if !realm.loginWithEmailAllowed>${msg("username")}<#elseif !realm.registrationEmailAsUsername>${msg("usernameOrEmail")}<#else>${msg("email")}</#if></label>

                                <input id="username" name="username" type="<#if realm.registrationEmailAsUsername>email<#else>text</#if>"
                                       class="${properties.kcInputClass!}"
                                       value="${(login.username!'')}"
                                       aria-invalid="<#if messagesPerField.existsError('username')>true</#if>"
                                       autofocus dir="ltr"
                                       autocomplete="${(enableWebAuthnConditionalUI?has_content)?then('username webauthn', 'username')}"/>

                                <#if messagesPerField.existsError('username')>
                                    <span id="input-error-username" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                                        ${kcSanitize(messagesPerField.get('username'))?no_esc}
                                    </span>
                                </#if>
                            </div>
                        </#if>

                        <#assign showRememberMe = realm.rememberMe && !usernameHidden??>
                        <#if showRememberMe || realm.resetPasswordAllowed>
                            <div class="kc-login-options">
                                <#if showRememberMe>
                                    <label class="kc-remember-me">
                                        <input id="rememberMe" name="rememberMe" type="checkbox"<#if login.rememberMe??> checked</#if>>
                                        ${msg("rememberMe")}
                                    </label>
                                <#else>
                                    <span></span>
                                </#if>
                                <#if realm.resetPasswordAllowed>
                                    <a id="kc-forgot-password" href="${url.loginResetCredentialsUrl}">${msg("doForgotPassword")}</a>
                                </#if>
                            </div>
                        </#if>

                        <div id="kc-form-buttons" class="${properties.kcFormGroupClass!}">
                            <input class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
                                   name="login" id="kc-login" type="submit" value="${msg("doLogIn")}"/>
                        </div>
                    </form>
                </#if>
            </div>
        </div>
        <#if enableWebAuthnConditionalUI?has_content>
            <div class="kc-divider" role="separator"><span>${msg("orDivider")}</span></div>
        </#if>
        <@passkeys.conditionalUIData />

    <#elseif section = "info" >
        <#if realm.password && realm.registrationAllowed && !registrationDisabled??>
            <div id="kc-registration">
                <span>${msg("noAccount")} <a href="${url.registrationUrl}">${msg("doRegister")}</a></span>
            </div>
        </#if>
    <#elseif section = "socialProviders" >
        <#if realm.password && social?? && social.providers?has_content>
            <div id="kc-social-providers" class="${properties.kcFormSocialAccountSectionClass!}">
                <ul class="${properties.kcFormSocialAccountListClass!}">
                    <#list social.providers as p>
                        <a data-once-link data-disabled-class="${properties.kcFormSocialAccountListButtonDisabledClass!}" id="social-${p.alias}"
                           class="${properties.kcFormSocialAccountListButtonClass!}" type="button" href="${p.loginUrl}">
                            <span class="${properties.kcFormSocialAccountNameClass!}">${p.displayName!}</span>
                        </a>
                    </#list>
                </ul>
            </div>
        </#if>
    </#if>
</@layout.registrationLayout>
