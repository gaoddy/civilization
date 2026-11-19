{{/*
Chart 名称。
*/}}
{{- define "civilization-server.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
公共标签。
*/}}
{{- define "civilization-server.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{ include "civilization-server.selectorLabels" . }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end -}}

{{/*
选择器标签；Service、Deployment 与 NetworkPolicy 依赖 app.kubernetes.io/name。
*/}}
{{- define "civilization-server.selectorLabels" -}}
app.kubernetes.io/name: {{ include "civilization-server.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}
