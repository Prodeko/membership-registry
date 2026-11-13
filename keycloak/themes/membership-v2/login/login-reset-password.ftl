<#--
  "Forgot password?" page: asks for the account email and sends a reset link.
  Keycloak answers with the same "check your email" message whether or not the
  address has an account, so this page never reveals who is a member.
  Styles: resources/css/login-extras.css
-->
<#import "template.ftl" as layout>
<@layout.registrationLayout displayInfo=false displayMessage=!messagesPerField.existsError('username'); section>
<!-- template: login-reset-password.ftl (membership-v2) -->
    <#if section = "header">
        ${msg("emailForgotTitle")}
    <#elseif section = "form">
        <p class="kc-page-intro">
            <#if realm.duplicateEmailsAllowed>${msg("emailInstructionUsername")}<#else>${msg("emailInstruction")}</#if>
        </p>

        <form id="kc-reset-password-form" onsubmit="login.disabled = true; return true;" action="${url.loginAction}" method="post">
            <div class="${properties.kcFormGroupClass!}">
                <label for="username" class="${properties.kcLabelClass!}"><#if !realm.loginWithEmailAllowed>${msg("username")}<#elseif !realm.registrationEmailAsUsername>${msg("usernameOrEmail")}<#else>${msg("email")}</#if></label>
                <input id="username" name="username" type="<#if realm.registrationEmailAsUsername>email<#else>text</#if>"
                       class="${properties.kcInputClass!}"
                       value="${(auth.attemptedUsername!'')}"
                       aria-invalid="<#if messagesPerField.existsError('username')>true</#if>"
                       autofocus autocomplete="email" dir="ltr"/>
                <#if messagesPerField.existsError('username')>
                    <span id="input-error-username" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                        ${kcSanitize(messagesPerField.get('username'))?no_esc}
                    </span>
                </#if>
            </div>

            <div id="kc-form-buttons" class="${properties.kcFormGroupClass!}">
                <input class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
                       name="login" type="submit" value="${msg("doSendResetLink")}"/>
            </div>
        </form>

        <div class="kc-back-link">
            <a href="${url.loginUrl}">${msg("backToLogin")}</a>
        </div>
    </#if>
</@layout.registrationLayout>
